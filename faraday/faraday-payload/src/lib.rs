//! The release's image list and the decoder that writes an image
//! (`docs/STARTER.md` §1.3): the `IMAGES` list the Starter is built with,
//! and the decoder that turns an image's `.xz` back into the image it
//! holds while it is written.
//!
//! Nothing here touches a disk or a file: a source is a `Read` (a
//! `Read + Seek` for a payload file, which is read twice) and a
//! destination is a `Write`, so every refusal is tested without a disk.
//!
//! - [`ImageList::parse`] reads `IMAGES`, the release's image list
//!   (`docs/IMAGES.md` §4), and refuses a malformed one, saying which
//!   line and why; [`ImageList::classify`]
//!   says from a file's name alone whether it is on the list, of
//!   another release, or neither.
//! - [`write_payload`] checks a payload file's `.xz` sum against its
//!   line in the list before a byte is written, then decompresses it into
//!   the writer as a stream, and succeeds only if what was written has
//!   the list's size and SHA-256.
//! - [`hash_image`] and [`write_image`] do the same for a developer's own
//!   image (`--image`, §1.1), a plain file or a `.xz`, which has no
//!   line in the list: the first computes the SHA-256 the Confirm page shows,
//!   the second writes the image and succeeds only if what it wrote has
//!   that sum.
//!
//! Decompression is `lzma-rust2`'s xz decoder, pure Rust with no
//! `unsafe` (`docs/deps/lzma-rust2.md`). It checks the xz format's own
//! integrity checks (each block's CRC-64 or other check, the index and
//! the footer), and its memory is the stream's dictionary, at most
//! [`MEMORY_LIMIT_KIB`]. No image is ever held in memory whole.

mod decode;
mod file;
mod images;

pub use decode::{
    ImageKind, ImageSum, MEMORY_LIMIT_KIB, PayloadError, hash_image, write_image, write_payload,
};
pub use file::PayloadFile;
pub use images::{ImageList, ImageListError, ImageListFault, ListedImage, NameClass, sha256_of};

use core::fmt;

/// A SHA-256 sum.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Sum(pub [u8; 32]);

impl Sha256Sum {
    /// Reads 64 lowercase hexadecimal digits, as `sha256sum` prints them.
    pub fn from_hex(text: &str) -> Option<Self> {
        // Upper case is refused: `sha256sum` writes lower case.
        if text.len() != 64 {
            return None;
        }
        let mut sum = [0u8; 32];
        base16ct::lower::decode(text, &mut sum).ok()?;
        Some(Self(sum))
    }
}

impl fmt::Display for Sha256Sum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; 64];
        let text = base16ct::lower::encode_str(&self.0, &mut buf).map_err(|_| fmt::Error)?;
        f.write_str(text)
    }
}

impl fmt::Debug for Sha256Sum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
