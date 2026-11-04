//! One payload file as the writer takes it (`docs/STARTER.md` §1.3): the
//! name of the `.xz`, its SHA-256, the name of what is inside it, that
//! file's SHA-256 and size, the build ID it carries and, for a stick
//! image, the SHA-256 of its boot partition. [`crate::ListedImage`]
//! makes one from a line of the release's image list.

use crate::Sha256Sum;

/// What a stick image's payload name starts with.
pub(crate) const STICK_PREFIX: &str = "faraday-stick-";

/// One payload file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadFile {
    /// The payload file's name, ending in `.xz`.
    pub name: String,
    /// The SHA-256 of the payload file, the `.xz` itself.
    pub sha256: Sha256Sum,
    /// The name of the image or binary inside it.
    pub inner_name: String,
    /// The SHA-256 of the image or binary inside it.
    pub inner_sha256: Sha256Sum,
    /// The size of the image or binary inside it, in bytes.
    pub inner_size: u64,
    /// The build ID the image or binary carries.
    pub build_id: String,
    /// The SHA-256 of the boot partition of a stick image (the
    /// `-boot.vfat` value `SHA256SUMS` lists); `None` for any other file.
    pub boot_sha256: Option<Sha256Sum>,
}

/// Decimal digits, no sign, no leading zero, above zero.
pub(crate) fn parse_size(text: &str) -> Option<u64> {
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
