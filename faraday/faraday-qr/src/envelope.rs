//! The Faraday file envelope (`faraday-file-v1`), byte for byte as
//! Faraday OS writes it (`lib/faraday/qr_transfer.py`): canonical JSON
//! with sorted keys and no spaces, holding a file's name, kind, size,
//! SHA-256 and base64 data. A SHA-256 match shows the file arrived whole,
//! not who sent it.

use faraday_bip::bitcoin::hashes::{Hash, sha256};

/// The largest file an envelope carries.
pub const MAX_FILE: usize = 256 * 1024;

/// The kinds Faraday OS writes.
pub const KINDS: [&str; 3] = ["file", "public-key", "signing-subkey"];

/// Why a file was not put in an envelope, or an envelope not read: a
/// kind, which the app words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The file is empty or over [`MAX_FILE`].
    Size,
    /// A kind Faraday OS does not write.
    Kind,
    /// A name that is not 1 to 128 letters, digits, dots, hyphens or
    /// underscores.
    Name,
    /// The envelope is over BBQr's [`crate::bbqr::MAX_DATA`].
    TooLarge,
    /// Not a Faraday file envelope.
    NotEnvelope,
    /// The envelope has no name.
    NoName,
    /// The envelope has no kind.
    NoKind,
    /// The envelope has no data.
    NoData,
    /// The envelope's data is not base64.
    NotBase64,
    /// The envelope's size, hash or fields do not match its file.
    Mismatch,
}

fn name_ok(name: &str) -> bool {
    let b = name.as_bytes();
    !b.is_empty()
        && b.len() <= 128
        && b[0].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(c))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A file in an envelope, as Faraday OS writes one.
pub fn pack(name: &str, data: &[u8], kind: &str) -> Result<Vec<u8>, Error> {
    if data.is_empty() || data.len() > MAX_FILE {
        return Err(Error::Size);
    }
    if !KINDS.contains(&kind) {
        return Err(Error::Kind);
    }
    if !name_ok(name) {
        return Err(Error::Name);
    }
    let digest = sha256::Hash::hash(data).to_byte_array();
    Ok(format!(
        "{{\"data\":\"{}\",\"format\":\"faraday-file-v1\",\"kind\":\"{kind}\",\"name\":\"{name}\",\"sha256\":\"{}\",\"size\":{}}}",
        faraday_bip::base64::encode(data),
        hex(&digest),
        data.len()
    )
    .into_bytes())
}

/// Whether bytes are offered as an envelope.
pub fn looks_like(raw: &[u8]) -> bool {
    raw.starts_with(b"{") && raw.windows(15).any(|w| w == b"faraday-file-v1")
}

/// The string value of `key` in flat JSON with no escapes, as `pack`
/// writes it.
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let at = text.find(&format!("\"{key}\":"))? + key.len() + 3;
    let rest = &text[at..];
    if let Some(r) = rest.strip_prefix('"') {
        r.split_once('"').map(|(v, _)| v)
    } else {
        Some(rest.split([',', '}']).next()?)
    }
}

/// The name, data and kind of an envelope. Its fields are read, the file
/// is put back in an envelope, and the two must be the same bytes: any
/// other field, any other order, a wrong size or hash is refused.
pub fn unpack(raw: &[u8]) -> Result<(String, Vec<u8>, String), Error> {
    if raw.len() > crate::bbqr::MAX_DATA {
        return Err(Error::TooLarge);
    }
    let text = std::str::from_utf8(raw).map_err(|_| Error::NotEnvelope)?;
    if field(text, "format") != Some("faraday-file-v1") {
        return Err(Error::NotEnvelope);
    }
    let name = field(text, "name").ok_or(Error::NoName)?;
    let kind = field(text, "kind").ok_or(Error::NoKind)?;
    let data = faraday_bip::base64::decode(field(text, "data").ok_or(Error::NoData)?)
        .map_err(|_| Error::NotBase64)?;
    let again = pack(name, &data, kind)?;
    if again != raw {
        return Err(Error::Mismatch);
    }
    Ok((name.to_string(), data, kind.to_string()))
}
