//! A BitLocker recovery key as plain text (`docs/HUB.md` §6.2, D18;
//! `docs/COMPANION.md` §4.6): the hub app shows it as a QR code a phone's
//! camera reads and offers to copy, and the Companion recognises it when
//! it is scanned or pasted. One writer and one reader, here, for both.
//!
//! ```text
//! BitLocker recovery key for DESKTOP-4K2Q, drive C:
//! 111111-222222-333333-444444-555555-666666-777777-888888
//! ```
//!
//! The first line names the computer and the drive; the second is the
//! key, eight groups of six digits. The words are a fixed format, as a
//! `ur:` prefix is, so a phone in any language reads the same text.
//! Everything that holds the key wipes it when dropped.

use std::fmt;

use zeroize::{Zeroize, Zeroizing};

/// The groups in a recovery key.
pub const GROUPS: usize = 8;
/// The digits in a group.
pub const DIGITS: usize = 6;

const HEAD: &str = "BitLocker recovery key for ";
const DRIVE: &str = ", drive ";

/// A recovery key: eight groups of six digits. `Debug` prints nothing of
/// it, and it is wiped when dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct RecoveryKey(Zeroizing<String>);

impl RecoveryKey {
    /// The key in `s` (groups separated by `-`, as Windows writes it), or
    /// `None` for anything else.
    pub fn parse(s: &str) -> Option<RecoveryKey> {
        let s = s.trim();
        if s.len() != GROUPS * DIGITS + GROUPS - 1 {
            return None;
        }
        let ok = s.split('-').count() == GROUPS
            && s.split('-')
                .all(|g| g.len() == DIGITS && g.bytes().all(|b| b.is_ascii_digit()));
        if !ok {
            return None;
        }
        let mut out = Zeroizing::new(String::with_capacity(s.len()));
        out.push_str(s);
        Some(RecoveryKey(out))
    }

    /// The key as Windows writes it.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Group `i` (0 to 7).
    pub fn group(&self, i: usize) -> &str {
        let at = i.min(GROUPS - 1) * (DIGITS + 1);
        &self.0[at..at + DIGITS]
    }
}

impl fmt::Debug for RecoveryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryKey(..)")
    }
}

/// What the text names: the computer, the drive and the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyText {
    /// The computer's name, as Windows names it.
    pub computer: String,
    /// The drive ("C:").
    pub drive: String,
    /// The key.
    pub key: RecoveryKey,
}

/// Whether `s` can stand in the text's first line: no line break, no
/// `", drive "`, not empty.
fn plain(s: &str) -> bool {
    !s.is_empty() && !s.contains(['\n', '\r']) && !s.contains(DRIVE)
}

impl KeyText {
    /// The text, wiped when dropped; `None` when the computer's or the
    /// drive's name cannot stand in it.
    pub fn write(&self) -> Option<Zeroizing<String>> {
        if !plain(&self.computer) || !plain(&self.drive) {
            return None;
        }
        let len = HEAD.len() + self.computer.len() + DRIVE.len() + self.drive.len() + 1;
        let mut out = Zeroizing::new(String::with_capacity(len + self.key.as_str().len()));
        out.push_str(HEAD);
        out.push_str(&self.computer);
        out.push_str(DRIVE);
        out.push_str(&self.drive);
        out.push('\n');
        out.push_str(self.key.as_str());
        Some(out)
    }

    /// The key text in `s`, as [`KeyText::write`] writes it (a line end
    /// may be `\r\n`, and the text may end with one), or `None`.
    pub fn read(s: &str) -> Option<KeyText> {
        let s = s.strip_suffix('\n').unwrap_or(s);
        let s = s.strip_suffix('\r').unwrap_or(s);
        let (first, key) = s.split_once('\n')?;
        let first = first.strip_suffix('\r').unwrap_or(first);
        let rest = first.strip_prefix(HEAD)?;
        let (computer, drive) = rest.rsplit_once(DRIVE)?;
        if !plain(computer) || !plain(drive) || key.contains('\n') {
            return None;
        }
        Some(KeyText {
            computer: computer.to_string(),
            drive: drive.to_string(),
            key: RecoveryKey::parse(key)?,
        })
    }
}

impl Zeroize for KeyText {
    fn zeroize(&mut self) {
        self.key.0.zeroize();
    }
}
