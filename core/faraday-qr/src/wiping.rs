//! The buffers a scanned file passes through, held in wiping memory: what
//! the files that read and write text need of them, so those files name no
//! secret type.

use std::io::Write;
use zeroize::Zeroizing;

/// Bytes that wipe themselves when dropped.
pub type Bytes = Zeroizing<Vec<u8>>;

/// The bytes moved out of the buffer, which is left empty; what is handed
/// on is not copied.
pub fn take(d: &mut Bytes) -> Vec<u8> {
    std::mem::take(&mut **d)
}

/// The bytes moved out when they are UTF-8; otherwise they stay in the
/// buffer, which wipes them.
pub fn take_utf8(d: &mut Bytes) -> Option<Vec<u8>> {
    std::str::from_utf8(d).ok()?;
    Some(take(d))
}

/// The envelope's JSON for `data`, in one buffer sized once, with the base64
/// of the file written into it where it goes. `None` if the text does not
/// fit the room kept for it.
pub fn envelope(data: &[u8], kind: &str, name: &str, digest: &str) -> Option<Bytes> {
    let plain = (data.len() * 4).div_ceil(3);
    let padded = data.len().div_ceil(3) * 4;
    // The fixed text, the digest and the size come to under 192.
    let mut out = Zeroizing::new(Vec::with_capacity(padded + 192 + kind.len() + name.len()));
    out.extend_from_slice(b"{\"data\":\"");
    let at = out.len();
    out.resize(at + padded, b'=');
    faraday_bip::base64::encode_into(data, &mut out[at..at + plain]);
    // not wording: the envelope's own fields
    write!(
        out,
        "\",\"format\":\"faraday-file-v1\",\"kind\":\"{kind}\",\"name\":\"{name}\",\"sha256\":\"{digest}\",\"size\":{}}}",
        data.len()
    )
    .ok()?;
    Some(out)
}
