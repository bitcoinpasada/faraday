//! KEF, Krux's encrypted code, as bytes and as text (`docs/KEF.md` §3, §5,
//! §8): the envelope is taken apart and put together, a text form is read
//! and written, and a scanned payload is recognised. Nothing here is
//! cryptographic and nothing here is costly; the keys and the ciphers are
//! `faraday-kef`'s.
//!
//! [`recognize`] is the application's classifier's last arm (the
//! application calls it after every other kind has declined a payload),
//! and its intake reaches it there.

use alloc::string::String;
use alloc::vec::Vec;

/// The twelve versions, in Krux's order (`kef.VERSIONS`).
pub const VERSIONS: [u8; 12] = [0, 1, 5, 6, 7, 10, 11, 12, 15, 16, 20, 21];

/// The most bytes an id has.
pub const ID_MAX: usize = 252;

/// The count `i` of 10,000 rounds at most, as Krux reads it.
const QUOTIENT_MAX: u32 = 10_000;

/// The largest count a code states: 3 bytes.
pub const COUNT_MAX_RAW: u32 = (1 << 24) - 1;

/// The cipher mode of a version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// AES-ECB, no IV.
    Ecb,
    /// AES-CBC, a 16-byte IV.
    Cbc,
    /// AES-CTR, a 12-byte IV.
    Ctr,
    /// AES-GCM, a 12-byte IV and a 4-byte tag.
    Gcm,
}

/// How a version pads before encrypting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Padding {
    /// Zero bytes to the block.
    Nul,
    /// PKCS #7.
    Pkcs7,
    /// None: a stream.
    None,
}

/// What a version is (`docs/KEF.md` §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    /// The version byte.
    pub version: u8,
    /// Krux's name for it.
    pub name: &'static str,
    /// The cipher mode.
    pub mode: Mode,
    /// The padding.
    pub padding: Padding,
    /// The check's length, and where it is: positive, 3 or 4 bytes after
    /// the ciphertext in the open (and for GCM the tag); negative, that
    /// many bytes inside the ciphertext, hidden.
    pub auth: i8,
    /// Whether the plaintext is deflated first.
    pub compressed: bool,
}

impl Spec {
    /// The bytes of IV the version carries.
    pub fn iv_len(&self) -> usize {
        match self.mode {
            Mode::Ecb => 0,
            Mode::Cbc => 16,
            Mode::Ctr | Mode::Gcm => 12,
        }
    }

    /// Versions 0 and 1, which the specification marks legacy.
    pub fn legacy(&self) -> bool {
        self.version <= 1
    }
}

const fn spec(
    version: u8,
    name: &'static str,
    mode: Mode,
    padding: Padding,
    auth: i8,
    compressed: bool,
) -> Spec {
    Spec {
        version,
        name,
        mode,
        padding,
        auth,
        compressed,
    }
}

const SPECS: [Spec; 12] = [
    spec(0, "AES-ECB v1", Mode::Ecb, Padding::Nul, -16, false),
    spec(1, "AES-CBC v1", Mode::Cbc, Padding::Nul, -16, false),
    spec(5, "AES-ECB", Mode::Ecb, Padding::Nul, 3, false),
    spec(6, "AES-ECB +p", Mode::Ecb, Padding::Pkcs7, -4, false),
    spec(7, "AES-ECB +c", Mode::Ecb, Padding::Pkcs7, -4, true),
    spec(10, "AES-CBC", Mode::Cbc, Padding::Nul, 4, false),
    spec(11, "AES-CBC +p", Mode::Cbc, Padding::Pkcs7, -4, false),
    spec(12, "AES-CBC +c", Mode::Cbc, Padding::Pkcs7, -4, true),
    spec(15, "AES-CTR", Mode::Ctr, Padding::None, -4, false),
    spec(16, "AES-CTR +c", Mode::Ctr, Padding::None, -4, true),
    spec(20, "AES-GCM", Mode::Gcm, Padding::None, 4, false),
    spec(21, "AES-GCM +c", Mode::Gcm, Padding::None, 4, true),
];

/// The version `v`, or `None` for a byte that is not a KEF version.
pub fn spec_of(v: u8) -> Option<&'static Spec> {
    SPECS.iter().find(|s| s.version == v)
}

/// An envelope taken apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// The id, which is the PBKDF2 salt.
    pub id: Vec<u8>,
    /// The version byte, one of [`VERSIONS`].
    pub version: u8,
    /// The PBKDF2 count, 10,000 to 100,000,000 or up to 16,777,215.
    pub count: u32,
    /// The rest: IV, ciphertext and an exposed check.
    pub payload: Vec<u8>,
}

impl Envelope {
    /// The version's description. Always there for an envelope that
    /// [`unwrap`] returned.
    pub fn spec(&self) -> &'static Spec {
        spec_of(self.version).expect("an envelope holds a known version")
    }
}

/// Why bytes are not an envelope, or an envelope cannot be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Too short to hold the header.
    Short,
    /// An id of more than 252 bytes.
    IdLength,
    /// A version byte that is not KEF's.
    Version,
    /// A count of 0, or not a count `wrap` writes.
    Count,
    /// A payload of the wrong size for its version.
    Payload,
}

/// The count a 3-byte `i` states: `i` times 10,000 up to 10,000, else `i`
/// itself. `None` for 0.
pub fn count_of(i: u32) -> Option<u32> {
    match i {
        0 => None,
        1..=QUOTIENT_MAX => Some(i * 10_000),
        _ => Some(i),
    }
}

/// The 3 bytes that state `count`, as `kef.wrap` writes it: the quotient
/// when `count` is a multiple of 10,000 from 10,000 to 100,000,000, else
/// the count itself when it is above 10,000 and under 2^24.
pub fn quotient_of(count: u32) -> Option<u32> {
    if count != 0 && count.is_multiple_of(10_000) {
        let q = count / 10_000;
        (1..=QUOTIENT_MAX).contains(&q).then_some(q)
    } else if count > 10_000 && count <= COUNT_MAX_RAW {
        Some(count)
    } else {
        None
    }
}

/// Whether `payload` has the size `v` needs (`docs/KEF.md` §4.2): after
/// the IV and an exposed check, whole blocks for ECB and CBC and at least
/// one, a byte for CTR and GCM, and more than a hidden check.
pub fn payload_fits(s: &Spec, payload: &[u8]) -> bool {
    let exposed = usize::from(s.auth.max(0) as u8);
    let hidden = usize::from((-s.auth).max(0) as u8);
    let Some(body) = payload.len().checked_sub(s.iv_len() + exposed) else {
        return false;
    };
    match s.mode {
        Mode::Ecb | Mode::Cbc => body >= 16 && body % 16 == 0 && body > hidden,
        Mode::Ctr | Mode::Gcm => body > hidden,
    }
}

/// Takes `bytes` apart as Krux's `unwrap` does, with the refusals of
/// `docs/KEF.md` §14: a count of 0, an id over 252 bytes, an unknown
/// version and a payload that cannot hold a ciphertext.
pub fn unwrap(bytes: &[u8]) -> Result<Envelope, Refusal> {
    let len_id = usize::from(*bytes.first().ok_or(Refusal::Short)?);
    if len_id > ID_MAX {
        return Err(Refusal::IdLength);
    }
    let version = *bytes.get(1 + len_id).ok_or(Refusal::Short)?;
    let s = spec_of(version).ok_or(Refusal::Version)?;
    let i = bytes.get(2 + len_id..5 + len_id).ok_or(Refusal::Short)?;
    let i = u32::from_be_bytes([0, i[0], i[1], i[2]]);
    let count = count_of(i).ok_or(Refusal::Count)?;
    if count < 10_000 {
        return Err(Refusal::Count);
    }
    let payload = &bytes[len_id + 5..];
    if !payload_fits(s, payload) {
        return Err(Refusal::Payload);
    }
    Ok(Envelope {
        id: bytes[1..1 + len_id].to_vec(),
        version,
        count,
        payload: payload.to_vec(),
    })
}

/// Puts an envelope together as `kef.wrap` does: the id at most 252
/// bytes, a known version, a count [`quotient_of`] can write, and a
/// payload of the size [`unwrap`] accepts.
pub fn wrap(id: &[u8], version: u8, count: u32, payload: &[u8]) -> Result<Vec<u8>, Refusal> {
    if id.len() > ID_MAX {
        return Err(Refusal::IdLength);
    }
    let s = spec_of(version).ok_or(Refusal::Version)?;
    let i = quotient_of(count).ok_or(Refusal::Count)?;
    if !payload_fits(s, payload) {
        return Err(Refusal::Payload);
    }
    let mut out = Vec::with_capacity(5 + id.len() + payload.len());
    out.push(id.len() as u8);
    out.extend_from_slice(id);
    out.push(version);
    out.extend_from_slice(&i.to_be_bytes()[1..]);
    out.extend_from_slice(payload);
    Ok(out)
}

/// How an id reads: as text when it is UTF-8 with no control character,
/// else `None` and the caller shows `0x` hex, as Krux does.
pub fn id_text(id: &[u8]) -> Option<&str> {
    let t = core::str::from_utf8(id).ok()?;
    (!t.chars().any(char::is_control)).then_some(t)
}

/// A form an envelope is carried in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// The envelope's own bytes.
    Raw,
    /// Hex, in one case.
    Hex,
    /// RFC 4648 base32 in upper case, without padding.
    Base32,
    /// Electrum's base43, the alphanumeric QR mode's characters.
    Base43,
    /// Base64 with padding.
    Base64,
}

/// The most characters of base43 text [`recognize`] reads: base43 is
/// worked out digit by digit, so its time grows with the square of the
/// text, and an envelope is far shorter (`docs/KEF.md` §7.1). Longer text
/// is not a code, and is not decoded.
pub const BASE43_MAX: usize = 8_192;

/// Base43's alphabet (`B43CHARS` in Krux's `baseconv.py`).
pub const B43_CHARS: &[u8; 43] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ$*+-./:";

/// Costs a division of the whole number per digit, so its time grows with
/// the square of the length. `bytes` in base43: Krux's `pure_python_base_encode` at 43, each
/// leading zero byte a leading `0`. Empty for no bytes.
pub fn base43_encode(bytes: &[u8]) -> String {
    // Big-endian number, repeatedly divided by 43.
    let mut digits: Vec<u8> = Vec::new();
    let mut number: Vec<u8> = bytes.to_vec();
    let lead = bytes.iter().take_while(|&&b| b == 0).count();
    let mut start = lead;
    while start < number.len() {
        let mut rem = 0u32;
        for b in &mut number[start..] {
            let cur = rem * 256 + u32::from(*b);
            *b = (cur / 43) as u8;
            rem = cur % 43;
        }
        digits.push(B43_CHARS[rem as usize]);
        while start < number.len() && number[start] == 0 {
            start += 1;
        }
    }
    digits.extend(core::iter::repeat_n(B43_CHARS[0], lead));
    digits.reverse();
    String::from_utf8(digits).expect("base43 characters are ASCII")
}

/// Costs a multiplication of the whole number per digit, so its time grows
/// with the square of the length. Bytes from base43 text, `None` for a character outside the alphabet.
/// Leading `0` characters are leading zero bytes.
pub fn base43_decode(text: &str) -> Option<Vec<u8>> {
    let lead = text.bytes().take_while(|&c| c == B43_CHARS[0]).count();
    // Little-endian bytes of the number.
    let mut out: Vec<u8> = Vec::new();
    for c in text.bytes() {
        let digit = B43_CHARS.iter().position(|&x| x == c)? as u32;
        let mut carry = digit;
        for b in &mut out {
            let cur = u32::from(*b) * 43 + carry;
            *b = cur as u8;
            carry = cur >> 8;
        }
        while carry > 0 {
            out.push(carry as u8);
            carry >>= 8;
        }
    }
    // The leading zero characters were digits 0 and added no byte.
    out.extend(core::iter::repeat_n(0u8, lead));
    out.reverse();
    Some(out)
}

const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// RFC 4648 base32 without padding, in upper case.
pub fn base32_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for &b in bytes {
        acc = (acc << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((acc >> bits) & 31) as usize] as char);
        }
        acc &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(B32[((acc << (5 - bits)) & 31) as usize] as char);
    }
    out
}

/// Bytes from RFC 4648 base32, upper case, with or without `=` padding.
pub fn base32_decode(text: &str) -> Option<Vec<u8>> {
    let text = text.trim_end_matches('=');
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = B32.iter().position(|&x| x == c)? as u32;
        acc = (acc << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 with padding.
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for k in 0..4 {
            if k <= chunk.len() {
                out.push(B64[((n >> (18 - 6 * k)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Bytes from base64; `None` for a character outside the alphabet or a
/// padding that is not the usual one.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let t = text.as_bytes();
    if !t.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::new();
    for quad in t.chunks(4) {
        let mut n = 0u32;
        let mut pad = 0;
        for (k, &c) in quad.iter().enumerate() {
            let v = if c == b'=' {
                if k < 2 {
                    return None;
                }
                pad += 1;
                0
            } else {
                if pad > 0 {
                    return None;
                }
                B64.iter().position(|&x| x == c)? as u32
            };
            n = (n << 6) | v;
        }
        for k in 0..3 - pad {
            out.push((n >> (16 - 8 * k)) as u8);
        }
    }
    Some(out)
}

/// `bytes` as text in `form`: upper-case hex for [`Form::Hex`]; `None`
/// for [`Form::Raw`], which is not text.
pub fn encode(form: Form, bytes: &[u8]) -> Option<String> {
    Some(match form {
        Form::Raw => return None,
        Form::Hex => {
            let mut s = String::with_capacity(bytes.len() * 2);
            for b in bytes {
                s.push(char::from_digit(u32::from(b >> 4), 16)?.to_ascii_uppercase());
                s.push(char::from_digit(u32::from(b & 15), 16)?.to_ascii_uppercase());
            }
            s
        }
        Form::Base32 => base32_encode(bytes),
        Form::Base43 => base43_encode(bytes),
        Form::Base64 => base64_encode(bytes),
    })
}

/// A text, read as `form`, and only if it writes back to the same text
/// (`docs/KEF.md` §5): hex in one case, base32 without padding, base43
/// and base64 as they are written.
fn read_text(form: Form, text: &str) -> Option<Vec<u8>> {
    let bytes = match form {
        Form::Raw => return None,
        Form::Base43 if text.len() > BASE43_MAX => return None,
        Form::Hex => crate::encodings::from_hex(text)?,
        Form::Base32 => base32_decode(text)?,
        Form::Base43 => base43_decode(text)?,
        Form::Base64 => base64_decode(text)?,
    };
    if bytes.is_empty() {
        return None;
    }
    let again = encode(form, &bytes)?;
    let same = if form == Form::Hex {
        again == text || again.to_ascii_lowercase() == text
    } else {
        again == text
    };
    same.then_some(bytes)
}

/// The forms a text might be, in Krux's order (`hint_encodings`), from its
/// least and greatest character.
fn hints(text: &str) -> Vec<Form> {
    let mut out = Vec::new();
    let (Some(min), Some(max)) = (text.chars().min(), text.chars().max()) else {
        return out;
    };
    if text.chars().count().is_multiple_of(2) && '0' <= min && max <= 'f' {
        out.push(Form::Hex);
    }
    if '2' <= min && max <= 'Z' {
        out.push(Form::Base32);
    }
    if '$' <= min && max <= 'Z' {
        out.push(Form::Base43);
    }
    if '+' <= min && max <= 'z' {
        out.push(Form::Base64);
    }
    out
}

/// A scanned payload that is KEF.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recognized {
    /// The form it was carried in.
    pub form: Form,
    /// The envelope's bytes, as `wrap` writes them.
    pub bytes: Vec<u8>,
    /// The envelope.
    pub envelope: Envelope,
}

/// Costs, for base43 text, as much as [`base43_decode`] and [`base43_encode`]
/// of up to [`BASE43_MAX`] characters. Whether a scanned `payload` is a KEF envelope: as UTF-8 text in the
/// forms Krux reads, in its order, the first that parses; else as the
/// envelope's own bytes. This is the weakest test the classifier has
/// (`docs/KEF.md` §8: an envelope has no magic number), so it is asked
/// last, after every stricter kind has said no, and a match only offers a
/// card; it never opens anything.
///
/// Raw 16 or 32 bytes are a CompactSeedQR first, and the caller says so
/// before asking: this function does not look for it.
pub fn recognize(payload: &[u8]) -> Option<Recognized> {
    if let Ok(text) = core::str::from_utf8(payload) {
        for form in hints(text) {
            if let Some(bytes) = read_text(form, text)
                && let Ok(envelope) = unwrap(&bytes)
            {
                return Some(Recognized {
                    form,
                    bytes,
                    envelope,
                });
            }
        }
    }
    let envelope = unwrap(payload).ok()?;
    Some(Recognized {
        form: Form::Raw,
        bytes: payload.to_vec(),
        envelope,
    })
}
