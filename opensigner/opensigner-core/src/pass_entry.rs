//! A passphrase typed for something this device encrypts: the entry
//! and the typed-twice pair the Backup flow, a note and a recovery sheet
//! show before [`osk_backup::oskb`] seals under it.
//!
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text.

use osk_bip::bip39::MAX_PASSPHRASE_BYTES;
use osk_crypto::{Secret, Zeroize, ZeroizeOnDrop};

/// Characters a backup passphrase needs before ✓ is live. Nothing else
/// about strength is judged here; what makes a good one is a Learn page.
pub const MIN_PASSPHRASE: usize = 8;

/// Milliseconds a typed character stays visible before it is masked
/// (`docs/PLANNING.md` §4.6).
const MASK_MS: u64 = 500;

/// One typed backup passphrase: a fixed array zeroized on drop, with
/// the last character visible for half a second the way every other
/// masked entry in the app is.
pub struct PassEntry {
    bytes: [u8; MAX_PASSPHRASE_BYTES],
    len: u16,
    typed_at: u64,
}

impl Zeroize for PassEntry {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.len = 0;
        self.typed_at = 0;
    }
}

impl Drop for PassEntry {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for PassEntry {}

impl Default for PassEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl PassEntry {
    /// Nothing typed.
    pub fn new() -> Self {
        PassEntry {
            bytes: [0; MAX_PASSPHRASE_BYTES],
            len: 0,
            typed_at: 0,
        }
    }

    /// Appends a printable ASCII character at time `now_ms`.
    pub fn push(&mut self, c: char, now_ms: u64) {
        let n = usize::from(self.len);
        if !c.is_ascii() || !(0x20..=0x7E).contains(&(c as u32)) || n >= MAX_PASSPHRASE_BYTES {
            return;
        }
        self.bytes[n] = c as u8;
        self.len += 1;
        self.typed_at = now_ms;
    }

    /// Deletes the last character.
    pub fn pop(&mut self) {
        if self.len > 0 {
            self.len -= 1;
            self.bytes[usize::from(self.len)] = 0;
        }
    }

    /// Characters typed.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether nothing is typed.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether ✓ is live.
    pub fn long_enough(&self) -> bool {
        self.len() >= MIN_PASSPHRASE
    }

    /// The bytes themselves, for the one call that stretches them.
    pub fn expose(&self) -> &[u8] {
        &self.bytes[..self.len()]
    }

    /// Whether two entries are the same, over the whole of both arrays
    /// rather than the typed part.
    pub fn same_as(&self, other: &PassEntry) -> bool {
        let a = Secret::new(self.bytes);
        let b = Secret::new(other.bytes);
        self.len == other.len && a == b
    }

    /// Takes what is typed, leaving the entry empty.
    pub fn take(&mut self) -> PassEntry {
        core::mem::take(self)
    }

    /// The last typed character while it is still unmasked at `now_ms`.
    pub fn visible_char(&self, now_ms: u64) -> Option<char> {
        if self.len == 0 || now_ms >= self.mask_deadline()? {
            return None;
        }
        Some(self.bytes[self.len() - 1] as char)
    }

    /// When the last typed character masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        (self.len > 0).then_some(self.typed_at + MASK_MS)
    }
}

/// One passphrase typed twice: the two entries a flow that seals
/// something shows, and which of them is on screen. The Backup flow
/// keeps its own pair inside itself; everything else that seals — a
/// note, a recovery sheet — holds one of these.
#[derive(Default)]
pub struct PassPair {
    pass: PassEntry,
    first: PassEntry,
    repeat: bool,
    mismatch: bool,
}

impl Zeroize for PassPair {
    fn zeroize(&mut self) {
        self.pass.zeroize();
        self.first.zeroize();
        self.repeat = false;
        self.mismatch = false;
    }
}

impl Drop for PassPair {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for PassPair {}

impl PassPair {
    /// Nothing typed, on the first of the two entries.
    pub fn new() -> Self {
        PassPair::default()
    }

    /// Whether the second typing is the one on screen.
    pub fn repeat(&self) -> bool {
        self.repeat
    }

    /// Characters typed on the entry now on screen.
    pub fn len(&self) -> usize {
        self.pass.len()
    }

    /// Whether nothing is typed.
    pub fn is_empty(&self) -> bool {
        self.pass.is_empty()
    }

    /// Whether ✓ is live.
    pub fn ready(&self) -> bool {
        self.pass.long_enough()
    }

    /// Whether the last repeat did not match the first typing.
    pub fn mismatch(&self) -> bool {
        self.mismatch
    }

    /// The last typed character while it is still unmasked at `now_ms`.
    pub fn visible_char(&self, now_ms: u64) -> Option<char> {
        self.pass.visible_char(now_ms)
    }

    /// When that character masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        self.pass.mask_deadline()
    }

    /// The passphrase itself, for the one call that stretches it.
    pub fn expose(&self) -> &[u8] {
        self.pass.expose()
    }

    /// Appends a character at time `now_ms`.
    pub fn push(&mut self, c: char, now_ms: u64) {
        self.pass.push(c, now_ms);
        self.mismatch = false;
    }

    /// Deletes the last character.
    pub fn pop(&mut self) {
        self.pass.pop();
    }

    /// ✓. `true` when the second typing matched the first and the
    /// caller should seal.
    pub fn done(&mut self) -> bool {
        if !self.pass.long_enough() {
            return false;
        }
        if !self.repeat {
            self.first = self.pass.take();
            self.mismatch = false;
            self.repeat = true;
            return false;
        }
        if self.pass.same_as(&self.first) {
            return true;
        }
        self.pass.zeroize();
        self.mismatch = true;
        false
    }

    /// The chevron: the repeat goes back to the first typing with both
    /// entries emptied, and `false` means leaving the entry entirely.
    pub fn back(&mut self) -> bool {
        if !self.repeat {
            return false;
        }
        self.zeroize();
        true
    }
}
