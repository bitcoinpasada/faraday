//! The inner header that begins the decrypted body, and the inner random
//! stream that encrypts the protected values of the XML document
//! (`docs/VAULT.md` §1.1, §7.1).

use alloc::vec::Vec;

use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher};
use faraday_crypto::Secret;
use sha2::{Digest, Sha512};
use zeroize::Zeroizing;

use super::header::put_field;
use super::{Error, Unsupported};

const INNER_END: u8 = 0;
const INNER_STREAM_ID: u8 = 1;
const INNER_STREAM_KEY: u8 = 2;
const INNER_BINARY: u8 = 3;

/// ChaCha20 as the inner random stream.
const STREAM_CHACHA20: u32 = 3;
/// Salsa20, KeePass 2's older inner stream; named and refused.
const STREAM_SALSA20: u32 = 2;

/// An attachment: a binary of the inner header, which entries refer to by
/// its position.
#[derive(Clone)]
pub struct Binary {
    /// Whether the file marks it for memory protection.
    pub protected: bool,
    /// Its bytes.
    pub data: Zeroizing<Vec<u8>>,
}

/// What the inner header holds.
pub(super) struct Inner {
    /// The inner stream's key, as long as the file states.
    pub key: Zeroizing<Vec<u8>>,
    pub binaries: Vec<Binary>,
    /// Where the XML document begins in the body.
    pub document_at: usize,
}

/// Reads the inner header at the start of `body`.
pub(super) fn read_inner(body: &[u8]) -> Result<Inner, Error> {
    let mut at = 0usize;
    let mut stream = None;
    let mut key: Option<Zeroizing<Vec<u8>>> = None;
    let mut binaries = Vec::new();
    loop {
        let id = *body.get(at).ok_or(Error::Damaged)?;
        let len = u32::from_le_bytes(
            body.get(at + 1..at + 5)
                .ok_or(Error::Damaged)?
                .try_into()
                .map_err(|_| Error::Damaged)?,
        ) as usize;
        at += 5;
        let end = at.checked_add(len).ok_or(Error::Damaged)?;
        let value = body.get(at..end).ok_or(Error::Damaged)?;
        at = end;
        match id {
            INNER_END if value.is_empty() => break,
            INNER_STREAM_ID => {
                let id = u32::from_le_bytes(value.try_into().map_err(|_| Error::Damaged)?);
                if stream.replace(id).is_some() {
                    return Err(Error::Damaged);
                }
            }
            INNER_STREAM_KEY => {
                if key.replace(Zeroizing::new(value.to_vec())).is_some() {
                    return Err(Error::Damaged);
                }
            }
            INNER_BINARY => {
                let (flags, data) = value.split_first().ok_or(Error::Damaged)?;
                binaries.push(Binary {
                    protected: flags & 1 != 0,
                    data: Zeroizing::new(data.to_vec()),
                });
            }
            INNER_END => return Err(Error::Damaged),
            other => return Err(Error::UnknownField(other)),
        }
    }
    match stream.ok_or(Error::Damaged)? {
        STREAM_CHACHA20 => {}
        STREAM_SALSA20 => return Err(Error::Unsupported(Unsupported::Salsa20)),
        _ => return Err(Error::Unsupported(Unsupported::OtherInnerStream)),
    }
    let key = key.ok_or(Error::Damaged)?;
    if key.is_empty() {
        return Err(Error::Damaged);
    }
    Ok(Inner {
        key,
        binaries,
        document_at: at,
    })
}

/// The inner header Faraday writes.
pub(super) fn write(key: &[u8], binaries: &[Binary], out: &mut Vec<u8>) {
    put_field(INNER_STREAM_ID, &STREAM_CHACHA20.to_le_bytes(), out);
    put_field(INNER_STREAM_KEY, key, out);
    for binary in binaries {
        out.push(INNER_BINARY);
        out.extend_from_slice(&(binary.data.len() as u32 + 1).to_le_bytes());
        out.push(u8::from(binary.protected));
        out.extend_from_slice(&binary.data);
    }
    put_field(INNER_END, &[], out);
}

/// The inner random stream: ChaCha20 under the first 32 bytes of the
/// SHA-512 of the inner key and a nonce of the next 12, as KeePass keys
/// it. A protected value is XORed with the next bytes of the stream, in
/// the order the values stand in the document, so encrypting and
/// decrypting are one call.
pub struct InnerStream(ChaCha20);

impl InnerStream {
    /// The stream for an inner key.
    pub fn new(key: &[u8]) -> Self {
        let wide = Secret::new(<[u8; 64]>::from(Sha512::digest(key)));
        let cipher = ChaCha20::new_from_slices(&wide.expose()[..32], &wide.expose()[32..44])
            .expect("a 32-byte key and a 12-byte nonce");
        Self(cipher)
    }

    /// XORs `data` with the stream's next bytes.
    pub fn apply(&mut self, data: &mut [u8]) {
        self.0.apply_keystream(data);
    }
}
