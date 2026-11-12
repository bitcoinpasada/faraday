//! The inner header that begins the decrypted body, and the inner random
//! stream that encrypts the protected values of the XML document
//! (`docs/VAULT.md` §1.1, §7.1).
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::Deref;

use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
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

/// A range of one wiping buffer that several holders share without copying
/// it (F206 point 6): the decrypted body an opened file read, of which
/// each attachment is a range. The XML document is in a buffer of its own,
/// wiped and freed once it is parsed.
pub struct Shared {
    buf: Arc<Zeroizing<Vec<u8>>>,
    start: usize,
    end: usize,
}

#[expect(
    clippy::disallowed_methods,
    reason = "a public value: its fields hold no secret"
)]
impl Clone for Shared {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            start: self.start,
            end: self.end,
        }
    }
}

impl Shared {
    /// The whole of `buf`.
    pub fn new(buf: Zeroizing<Vec<u8>>) -> Self {
        let end = buf.len();
        Self {
            buf: Arc::new(buf),
            start: 0,
            end,
        }
    }

    /// Bytes `start..end` of the same buffer, if they lie inside this one.
    #[expect(
        clippy::disallowed_methods,
        reason = "a second handle on the same buffer, not a second copy of it"
    )]
    pub(super) fn range(&self, start: usize, end: usize) -> Option<Self> {
        let (from, to) = (self.start.checked_add(start)?, self.start.checked_add(end)?);
        (start <= end && to <= self.end).then(|| Self {
            buf: Arc::clone(&self.buf),
            start: from,
            end: to,
        })
    }
}

#[cfg(test)]
impl Shared {
    /// The length of the whole buffer this is a range of: what keeps
    /// being held while this is.
    pub(super) fn backing_len(&self) -> usize {
        self.buf.len()
    }
}

impl Deref for Shared {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.buf[self.start..self.end]
    }
}

impl From<Vec<u8>> for Shared {
    fn from(bytes: Vec<u8>) -> Self {
        Self::new(Zeroizing::new(bytes))
    }
}

impl From<Zeroizing<Vec<u8>>> for Shared {
    fn from(buf: Zeroizing<Vec<u8>>) -> Self {
        Self::new(buf)
    }
}

/// An attachment: a binary of the inner header, which entries refer to by
/// its position. Its bytes are a range of the body the file was read into.
pub struct Binary {
    /// Whether the file marks it for memory protection.
    pub protected: bool,
    /// Its bytes.
    pub data: Shared,
}

#[expect(
    clippy::disallowed_methods,
    reason = "a public value: its fields hold no secret"
)]
impl Clone for Binary {
    fn clone(&self) -> Self {
        Self {
            protected: self.protected,
            data: self.data.clone(),
        }
    }
}

/// The attachments a seal writes: those read, with the ones nothing refers
/// to emptied in place (bytes gone, protection flag and position kept) and
/// the trailing emptied ones left off. Borrows the bytes, copies none.
pub struct Binaries<'a> {
    all: &'a [Binary],
    /// How many are written.
    len: usize,
    /// Per position of `all`: written empty. Empty when none is.
    emptied: Vec<bool>,
}

impl<'a> Binaries<'a> {
    /// All of `all`, as read.
    pub fn all(all: &'a [Binary]) -> Self {
        Self {
            all,
            len: all.len(),
            emptied: Vec::new(),
        }
    }

    /// `all` with the positions `used` does not mark emptied and the
    /// trailing ones left off. `used` has one entry per attachment.
    pub(super) fn used(all: &'a [Binary], used: &[bool]) -> Self {
        let len = used.iter().rposition(|&u| u).map_or(0, |last| last + 1);
        let emptied: Vec<bool> = used[..len].iter().map(|&u| !u).collect();
        Self { all, len, emptied }
    }

    /// Each written attachment: whether it is protected, and its bytes.
    pub fn iter(&self) -> impl Iterator<Item = (bool, &[u8])> + '_ {
        self.all[..self.len].iter().enumerate().map(|(i, b)| {
            let gone = self.emptied.get(i).copied().unwrap_or(false);
            (b.protected, if gone { &[][..] } else { &b.data[..] })
        })
    }

    /// How many are written.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether none is written.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// What the inner header holds.
pub(super) struct Inner {
    /// The inner stream's key, as long as the file states.
    pub key: Zeroizing<Vec<u8>>,
    pub binaries: Vec<Binary>,
    /// Where the XML document begins in the body.
    pub document_at: usize,
}

/// Reads the inner header at the start of `body`; each attachment is a
/// range of `body`, not a copy. The tests' way in; a file is read by
/// [`scan`], which leaves the document out of the shared body.
#[cfg(test)]
pub(super) fn read_inner(body: &Shared) -> Result<Inner, Error> {
    scan(body)?.into_inner(body)
}

/// What the inner header holds, with each attachment as the range of the
/// body it is at.
pub(super) struct Scanned {
    pub key: Zeroizing<Vec<u8>>,
    /// Whether the file marks it for memory protection, and where its bytes
    /// are in the body.
    pub binaries: Vec<(bool, core::ops::Range<usize>)>,
    pub document_at: usize,
    /// How much of the body is kept: up to where the end field's value
    /// starts, so that value costs nothing once read.
    pub kept: usize,
}

impl Scanned {
    /// The same with each attachment a range of `body`, which is the body
    /// scanned.
    pub(super) fn into_inner(self, body: &Shared) -> Result<Inner, Error> {
        let mut binaries = Vec::new();
        binaries
            .try_reserve_exact(self.binaries.len())
            .map_err(|_| Error::TooLarge)?;
        for (protected, r) in self.binaries {
            binaries.push(Binary {
                protected,
                data: body.range(r.start, r.end).ok_or(Error::Damaged)?,
            });
        }
        Ok(Inner {
            key: self.key,
            binaries,
            document_at: self.document_at,
        })
    }
}

/// Reads the inner header at the start of `body`, as [`read_inner`] does,
/// but gives where each attachment is instead of sharing the body.
pub(super) fn scan(body: &[u8]) -> Result<Scanned, Error> {
    let mut at = 0usize;
    let mut stream = None;
    let mut key: Option<Zeroizing<Vec<u8>>> = None;
    let mut binaries = Vec::new();
    let kept;
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
        let value_at = at;
        at = end;
        match id {
            // KeePass and KeePassXC ignore the end field's bytes; KeeWeb writes four.
            INNER_END => {
                kept = value_at;
                break;
            }
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
                let flags = *value.first().ok_or(Error::Damaged)?;
                if binaries.len() >= super::MAX_BINARIES {
                    return Err(Error::TooLarge);
                }
                binaries.try_reserve(1).map_err(|_| Error::TooLarge)?;
                binaries.push((flags & 1 != 0, end - len + 1..end));
            }
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
    Ok(Scanned {
        key,
        binaries,
        document_at: at,
        kept,
    })
}

/// The inner header Faraday writes, handed to `put` in pieces: an
/// attachment's bytes go to it as they are, never copied into a buffer.
pub(super) fn write(key: &[u8], binaries: &Binaries<'_>, put: &mut impl FnMut(&[u8])) {
    // The key is in it: a buffer that wipes.
    let mut head = Zeroizing::new(Vec::new());
    put_field(INNER_STREAM_ID, &STREAM_CHACHA20.to_le_bytes(), &mut head);
    put_field(INNER_STREAM_KEY, key, &mut head);
    put(&head);
    for (protected, data) in binaries.iter() {
        let mut lead = [0u8; 6];
        lead[0] = INNER_BINARY;
        lead[1..5].copy_from_slice(&(data.len() as u32 + 1).to_le_bytes());
        lead[5] = u8::from(protected);
        put(&lead);
        put(data);
    }
    head.clear();
    put_field(INNER_END, &[], &mut head);
    put(&head);
}

/// The length [`write`] gives, worked out without writing.
pub(super) fn written_len(key: &[u8], binaries: &Binaries<'_>) -> usize {
    // Each field: an id, a length and the value.
    let fixed = (5 + 4) + (5 + key.len()) + 5;
    binaries.iter().fold(fixed, |sum, (_, data)| {
        sum.saturating_add(5 + 1 + data.len())
    })
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

    /// How many bytes of the stream have been used.
    pub fn position(&self) -> u64 {
        self.0.current_pos()
    }

    /// Goes back to `position`, to read again what was read since.
    pub fn rewind(&mut self, position: u64) {
        self.0.seek(position);
    }
}
