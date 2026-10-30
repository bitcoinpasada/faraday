//! Which Faraday a stick's boot partition holds: the kernel's release
//! string, found in the partition's bytes (`docs/DECISIONS.md` F5,
//! `docs/STARTER.md` §4.1). Faraday Starter reads a stick's version with
//! [`find_release`].
//!
//! The image build puts the Faraday version and commit in the kernel's
//! release string (`CONFIG_LOCALVERSION`). The PC stick's bzImage carries
//! that string uncompressed in its setup code; the Pi card's kernel is
//! compressed, so its boot partition carries the string in a file of its
//! own, `faraday-build.txt`, a line. A stick made before Faraday put one
//! there (0.1.0 and earlier) has none.
//!
//! It reads nothing itself: the caller hands it the bytes. It uses std
//! alone, so the crate builds for every system the Starter runs on.
//!
//! [`layout`] says what makes a disk a Faraday stick or card and what
//! marks a boot partition, and [`medium`] how the device records the
//! medium it started from (F174, `docs/STICK-IDENTITY.md` §5): the device
//! finds its own stick with [`holds_release`].

pub mod layout;
pub mod medium;

/// What the image build puts in the kernel's release string before the
/// Faraday version: `6.6.84-faraday-0.2.0+4d0680b1a2b3`.
pub const MARK: &[u8] = b"-faraday-";

/// The longest release string taken: the kernel's own limit.
pub const MAX_RELEASE: usize = 64;

/// The largest boot partition searched: the PC's and the Pi's are both
/// 64 MiB (F80); an older stick's is 48 MiB and an older Pi card's
/// 32 MiB.
pub const MAX_PARTITION: u64 = 64 << 20;

/// A byte a kernel release string may hold.
fn release_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'+' | b'-' | b'_')
}

/// The first Faraday release string in `hay`: a whole string of release
/// bytes with [`MARK`] in it, starting with a digit, no longer than
/// [`MAX_RELEASE`], and ending inside `hay`.
pub fn find_release(hay: &[u8]) -> Option<String> {
    let mut from = 0;
    while from + MARK.len() <= hay.len() {
        let at = from + hay[from..].windows(MARK.len()).position(|w| w == MARK)?;
        let mut start = at;
        while start > 0 && at - start < MAX_RELEASE && release_byte(hay[start - 1]) {
            start -= 1;
        }
        let mut end = at + MARK.len();
        while end < hay.len() && end - start <= MAX_RELEASE && release_byte(hay[end]) {
            end += 1;
        }
        let whole = (start == 0 || !release_byte(hay[start - 1]))
            && end < hay.len()
            && !release_byte(hay[end])
            && end - start <= MAX_RELEASE;
        if whole && hay[start].is_ascii_digit() && end > at + MARK.len() {
            return String::from_utf8(hay[start..end].to_vec()).ok();
        }
        from = at + 1;
    }
    None
}

/// Whether `hay` holds the release string `want` as a whole Faraday
/// release string, anywhere in it: each one [`find_release`] finds is
/// compared in turn. Costs a search of `hay`.
#[must_use]
pub fn holds_release(hay: &[u8], want: &str) -> bool {
    let mut rest = hay;
    while let Some(found) = find_release(rest) {
        if found == want {
            return true;
        }
        // Past the string just found: it is in `rest`, whole.
        let Some(at) = rest
            .windows(found.len())
            .position(|w| w == found.as_bytes())
        else {
            return false;
        };
        rest = &rest[at + found.len()..];
    }
    false
}
