//! The hub app's payload (`docs/HUB.md` §1.3): the release's `.xz` files
//! carried inside the hub app, the `MANIFEST` that names each with its
//! two SHA-256 sums, and the decoder that turns a payload file back into
//! the image or binary it holds while it is written.
//!
//! Nothing here touches a disk or a file: a source is a `Read` (a
//! `Read + Seek` for a payload file, which is read twice) and a
//! destination is a `Write`, so every refusal is tested without a disk.
//!
//! - [`Manifest::parse`] reads `MANIFEST` and refuses a malformed one,
//!   saying which line and why.
//! - [`write_payload`] checks a payload file's `.xz` sum against its
//!   manifest line before a byte is written, then decompresses it into
//!   the writer as a stream, and succeeds only if what was written has
//!   the manifest's size and SHA-256.
//! - [`hash_image`] and [`write_image`] do the same for a developer's own
//!   image (`--image`, §1.1), a plain file or a `.xz`, which has no
//!   manifest: the first computes the SHA-256 the Confirm page shows,
//!   the second writes the image and succeeds only if what it wrote has
//!   that sum.
//!
//! Decompression is `lzma-rust2`'s xz decoder, pure Rust with no
//! `unsafe` (`docs/deps/lzma-rust2.md`). It checks the xz format's own
//! integrity checks (each block's CRC-64 or other check, the index and
//! the footer), and its memory is the stream's dictionary, at most
//! [`MEMORY_LIMIT_KIB`]. No image is ever held in memory whole.

mod decode;
mod manifest;

pub use decode::{
    ImageKind, ImageSum, MEMORY_LIMIT_KIB, PayloadError, hash_image, write_image, write_payload,
};
pub use manifest::{Manifest, ManifestError, ManifestFault, PayloadFile};

use core::fmt;

/// A SHA-256 sum.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Sum(pub [u8; 32]);

impl Sha256Sum {
    /// Reads 64 lowercase hexadecimal digits, as `sha256sum` prints them.
    pub fn from_hex(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 64 {
            return None;
        }
        let mut sum = [0u8; 32];
        for (out, pair) in sum.iter_mut().zip(bytes.chunks_exact(2)) {
            *out = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
        }
        Some(Self(sum))
    }
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

impl fmt::Display for Sha256Sum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Sha256Sum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
