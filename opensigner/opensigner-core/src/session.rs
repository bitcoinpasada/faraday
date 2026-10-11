//! Session security (`docs/PLANNING.md` §5.2, §8.5 #1, #2 and #6,
//! §16.21; UX.md I1–I4): the session key every sealed secret is
//! encrypted under, the session PIN, the lock state, and the auto-lock and
//! auto-wipe timers.
//!
//! The PIN lives only in RAM. What the lock screen checks against is a
//! verifier: a salt derived from the session key when the PIN is set,
//! and HMAC-SHA512 of the digits under that salt. Eight wrong attempts
//! wipe the session. The PIN protects the lock screen of this session
//! and nothing else; it is not a backup and is never written anywhere.
//!
//! The digits themselves are kept beside the verifier, because the
//! session PIN is also the storage PIN of a key kept on the device
//! (`keep.rs`), which has to derive a key from the digits rather than
//! from a hash of them; a wipe drops them with everything else.
//!
//! Both are sealed under the session key, and neither has a plaintext
//! form (§16.51): four to eight digits behind one fast hash are the PIN
//! itself to anyone reading a memory image, and the kept key's Argon2id
//! and its element's failure counter are what that would walk around. A
//! PIN is set only after the shell has answered the entropy request
//! (§16.48), so the key that seals them is never weak.
//!
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text.

use osk_crypto::{
    KEY_LEN, Sealed, SealedBytes, Secret, SessionKey, Zeroize, ZeroizeOnDrop, hmac_sha512,
};

/// Fewest PIN digits.
pub const PIN_MIN: usize = 4;
/// Most PIN digits.
pub const PIN_MAX: usize = 8;
/// Wrong PINs allowed before the session wipes: the same count as the
/// stored key's, so that one PIN pad has one number (§16.65).
pub const ATTEMPTS: u8 = 8;
/// The auto-lock choices offered in Settings, in milliseconds.
pub const LOCK_OPTIONS: [u64; 4] = [30_000, 120_000, 300_000, 900_000];
/// The auto-wipe choices offered in Settings, in milliseconds; `None` is
/// "never".
pub const WIPE_OPTIONS: [Option<u64>; 5] = [
    Some(300_000),
    Some(600_000),
    Some(1_800_000),
    Some(3_600_000),
    None,
];
/// Default auto-lock: two minutes.
pub const DEFAULT_LOCK_MS: u64 = 120_000;
/// Default auto-wipe: ten minutes.
pub const DEFAULT_WIPE_MS: u64 = 600_000;
/// The lock countdown is shown when the lock is closer than this.
pub const STRIP_MS: u64 = 60_000;

const SALT_LABEL: &[u8] = b"osk-session-pin-salt";
const SCRAMBLE_LABEL: &[u8] = b"osk-pin-pad-scramble";

/// Digits typed on a PIN pad. Fixed-size, zeroized on drop.
pub struct PinEntry {
    digits: [u8; PIN_MAX],
    len: u8,
}

impl PinEntry {
    /// Nothing typed.
    pub const fn new() -> Self {
        PinEntry {
            digits: [0; PIN_MAX],
            len: 0,
        }
    }

    /// Appends a digit; other characters and a full entry are refused.
    pub fn push(&mut self, c: char) -> bool {
        let n = usize::from(self.len);
        if !c.is_ascii_digit() || n >= PIN_MAX {
            return false;
        }
        self.digits[n] = c as u8;
        self.len += 1;
        true
    }

    /// Removes the last digit.
    pub fn pop(&mut self) {
        if self.len > 0 {
            self.len -= 1;
            self.digits[usize::from(self.len)] = 0;
        }
    }

    /// Digits typed so far.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether nothing is typed.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the entry is long enough to be a PIN.
    pub fn is_complete(&self) -> bool {
        self.len() >= PIN_MIN
    }

    /// Wipes the digits.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// Constant-time equality of two entries. The lengths are compared
    /// first and short-circuit, so what this hides is the digits and not
    /// how many there are: two entries of different lengths are
    /// different in the time the comparison takes, and only entries of
    /// the same length reach the constant-time comparison. How many
    /// digits someone typed is visible on the pad as they type it, so it
    /// is not held secret here either.
    pub fn same_as(&self, other: &PinEntry) -> bool {
        let a = Secret::new(self.digits);
        let b = Secret::new(other.digits);
        self.len == other.len && a == b
    }

    /// The digits typed, as bytes.
    pub fn digits(&self) -> &[u8] {
        &self.digits[..self.len()]
    }
}

impl Default for PinEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for PinEntry {
    fn zeroize(&mut self) {
        self.digits.zeroize();
        self.len = 0;
    }
}

impl Drop for PinEntry {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for PinEntry {}

/// The digits in sealable form: the array and how much of it counts.
struct PinBytes {
    digits: [u8; PIN_MAX],
    len: u8,
}

impl Zeroize for PinBytes {
    fn zeroize(&mut self) {
        self.digits.zeroize();
        self.len = 0;
    }
}

impl SealedBytes for PinBytes {
    type Bytes = [u8; PIN_MAX + 1];
    const ZEROED: [u8; PIN_MAX + 1] = [0; PIN_MAX + 1];

    fn write_bytes(&self, out: &mut [u8; PIN_MAX + 1]) {
        out[..PIN_MAX].copy_from_slice(&self.digits);
        out[PIN_MAX] = self.len;
    }

    fn from_bytes(bytes: &[u8; PIN_MAX + 1]) -> Self {
        let mut digits = [0u8; PIN_MAX];
        digits.copy_from_slice(&bytes[..PIN_MAX]);
        PinBytes {
            digits,
            len: bytes[PIN_MAX],
        }
    }
}

/// The digits, sealed under the session key.
struct HeldPin(Sealed<PinBytes>);

impl HeldPin {
    fn of(key: &mut SessionKey, pin: &PinEntry) -> Option<Self> {
        let bytes = PinBytes {
            digits: pin.digits,
            len: pin.len,
        };
        Sealed::seal(key, bytes).ok().map(HeldPin)
    }

    fn with<R>(&self, key: &SessionKey, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        self.0
            .with(key, |p: &PinBytes| {
                f(&p.digits[..usize::from(p.len).min(PIN_MAX)])
            })
            .ok()
    }

    fn reseal(&mut self, old: &SessionKey, new: &mut SessionKey) {
        if let Ok(sealed) = self.0.reseal(old, new) {
            self.0 = sealed;
        }
    }
}

/// The verifier in sealable form: the salt, then the HMAC of the digits
/// under it.
struct PinBits {
    salt: [u8; 32],
    hash: [u8; 64],
}

impl Zeroize for PinBits {
    fn zeroize(&mut self) {
        self.salt.zeroize();
        self.hash.zeroize();
    }
}

impl SealedBytes for PinBits {
    type Bytes = [u8; 96];
    const ZEROED: [u8; 96] = [0; 96];

    fn write_bytes(&self, out: &mut [u8; 96]) {
        out[..32].copy_from_slice(&self.salt);
        out[32..].copy_from_slice(&self.hash);
    }

    fn from_bytes(bytes: &[u8; 96]) -> Self {
        let mut salt = [0u8; 32];
        let mut hash = [0u8; 64];
        salt.copy_from_slice(&bytes[..32]);
        hash.copy_from_slice(&bytes[32..]);
        PinBits { salt, hash }
    }
}

/// The stored form of the PIN: a per-session salt and the HMAC of the
/// digits under it, sealed together under the session key. Salt and
/// hash are only ever used together, so they are one sealed value.
struct PinHash(Sealed<PinBits>);

impl PinHash {
    fn of(key: &mut SessionKey, pin: &PinEntry) -> Option<Self> {
        let derived = key.derive(SALT_LABEL);
        let mut salt = [0u8; 32];
        salt.copy_from_slice(&derived.expose()[..32]);
        let bits = PinBits {
            salt,
            hash: hmac_sha512(&salt, pin.digits()),
        };
        salt.zeroize();
        Sealed::seal(key, bits).ok().map(PinHash)
    }

    /// Unseals to compare, at the rate a person types a PIN. The
    /// comparison itself is constant-time.
    fn matches(&self, key: &SessionKey, pin: &PinEntry) -> bool {
        self.0
            .with(key, |b: &PinBits| {
                Secret::new(hmac_sha512(&b.salt, pin.digits())) == Secret::new(b.hash)
            })
            .unwrap_or(false)
    }

    fn reseal(&mut self, old: &SessionKey, new: &mut SessionKey) {
        if let Ok(sealed) = self.0.reseal(old, new) {
            self.0 = sealed;
        }
    }
}

/// What entering a PIN on the lock screen led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unlock {
    /// The PIN was right; the session is unlocked.
    Unlocked,
    /// Wrong; this many attempts remain.
    Wrong(u8),
    /// Wrong for the last time; the caller wipes the session.
    Wiped,
}

/// The session: key, PIN, lock state, timers and their settings.
pub struct Session {
    key: SessionKey,
    pin: Option<PinHash>,
    /// The digits behind that hash, for the storage PIN of a kept key.
    digits: Option<HeldPin>,
    entry: PinEntry,
    attempts_left: u8,
    locked: bool,
    last_input_ms: u64,
    lock_after_ms: u64,
    wipe_after_ms: Option<u64>,
    scramble_pin: bool,
    /// A `RequestEntropy` is outstanding.
    entropy_requested: bool,
}

impl Session {
    /// A fresh session with a weak key mixed from `material` and the
    /// default settings.
    ///
    /// `docs/DESIGN.md` §4.3: "The pad is in digit order unless the
    /// 'Shuffle PIN pad' setting is on; it is off by default, because
    /// nothing on the pad screen can explain a shuffle." A person who
    /// wants the shuffle against a host that logs click positions turns
    /// it on in Settings, where the row says what it does.
    pub fn new(material: &[u8]) -> Self {
        Session {
            key: SessionKey::weak(material),
            pin: None,
            digits: None,
            entry: PinEntry::new(),
            attempts_left: ATTEMPTS,
            locked: false,
            last_input_ms: 0,
            lock_after_ms: DEFAULT_LOCK_MS,
            wipe_after_ms: Some(DEFAULT_WIPE_MS),
            scramble_pin: false,
            entropy_requested: false,
        }
    }

    // ----- key -----

    /// The session key.
    pub fn key(&self) -> &SessionKey {
        &self.key
    }

    /// The session key, for sealing.
    pub fn key_mut(&mut self) -> &mut SessionKey {
        &mut self.key
    }

    /// Whether the key was built without shell entropy.
    pub fn is_weak(&self) -> bool {
        self.key.is_weak()
    }

    /// Re-mixes a weak key from better `material` (the display
    /// parameters, once known). Does nothing once real entropy arrived.
    pub fn remix_weak(&mut self, material: &[u8]) {
        if self.key.is_weak() && self.pin.is_none() {
            self.key = SessionKey::weak(material);
        }
    }

    /// Notes that entropy is being requested. `false` when a request is
    /// already outstanding, so the caller emits nothing.
    pub fn request_entropy(&mut self) -> bool {
        if self.entropy_requested {
            return false;
        }
        self.entropy_requested = true;
        true
    }

    /// Whether an entropy request is unanswered.
    pub fn entropy_pending(&self) -> bool {
        self.entropy_requested
    }

    /// Consumes an entropy answer: the key rotates to one built from
    /// `bytes`, and the old key is returned so that the caller can
    /// re-seal everything under the new one before dropping it. Unasked
    /// entropy is ignored (`None`).
    pub fn entropy(&mut self, bytes: &[u8; KEY_LEN]) -> Option<SessionKey> {
        if !self.entropy_requested {
            return None;
        }
        self.entropy_requested = false;
        let mut old = SessionKey::from_entropy(bytes);
        core::mem::swap(&mut self.key, &mut old);
        Some(old)
    }

    /// The seed the PIN pad is scrambled with, when scrambling is on.
    /// Derived from the session key, so it changes when the key rotates.
    pub fn scramble_seed(&self) -> Option<u32> {
        if !self.scramble_pin {
            return None;
        }
        let d = self.key.derive(SCRAMBLE_LABEL);
        let b = d.expose();
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    // ----- PIN -----

    /// Whether a PIN is set.
    pub fn has_pin(&self) -> bool {
        self.pin.is_some()
    }

    /// Seals the verifier of `pin` and the digits behind it under the
    /// session key, and resets the attempt count. `false`, and no PIN
    /// set, when that key cannot seal: neither the verifier nor the
    /// digits are ever held in the clear.
    pub fn set_pin(&mut self, pin: &PinEntry) -> bool {
        let (Some(hash), Some(digits)) = (
            PinHash::of(&mut self.key, pin),
            HeldPin::of(&mut self.key, pin),
        ) else {
            return false;
        };
        self.pin = Some(hash);
        self.digits = Some(digits);
        self.attempts_left = ATTEMPTS;
        true
    }

    /// Runs `f` on the session PIN's digits, which are also the storage
    /// PIN of a key kept on the device. `None` when no PIN is set or the
    /// digits do not open under the session key.
    pub fn with_pin<R>(&self, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        self.digits.as_ref()?.with(&self.key, f)
    }

    /// Re-seals the PIN — the verifier and the digits both — after the
    /// session key rotated from `old`.
    pub fn reseal(&mut self, old: &SessionKey) {
        let Session {
            key, pin, digits, ..
        } = self;
        if let Some(p) = pin {
            p.reseal(old, key);
        }
        if let Some(d) = digits {
            d.reseal(old, key);
        }
    }

    /// Wrong attempts left before the session wipes.
    pub fn attempts_left(&self) -> u8 {
        self.attempts_left
    }

    /// Whether the lock screen is showing.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Locks: the lock screen shows until the PIN is entered. Nothing
    /// happens without a PIN.
    pub fn lock(&mut self) {
        if self.pin.is_some() {
            self.locked = true;
            self.entry.clear();
        }
    }

    /// The PIN typed on the lock screen so far.
    pub fn entry(&self) -> &PinEntry {
        &self.entry
    }

    /// Types a digit on the lock screen.
    pub fn entry_push(&mut self, c: char) -> bool {
        self.entry.push(c)
    }

    /// Deletes a digit on the lock screen.
    pub fn entry_pop(&mut self) {
        self.entry.pop();
    }

    /// Checks the typed PIN. The entry is wiped whatever the outcome.
    pub fn submit(&mut self) -> Unlock {
        if self.try_unlock() {
            return Unlock::Unlocked;
        }
        self.attempts_left = self.attempts_left.saturating_sub(1);
        if self.attempts_left == 0 {
            Unlock::Wiped
        } else {
            Unlock::Wrong(self.attempts_left)
        }
    }

    /// The PIN typed on the lock screen, checked and cleared; `true`
    /// unlocks. A wrong PIN costs nothing here: on a device that keeps
    /// a key the stored key's own count is the one that runs, and the
    /// blob is asked next (§16.63). [`submit`](Self::submit) is the lock
    /// screen everywhere else.
    pub fn try_unlock(&mut self) -> bool {
        let ok = self.entry.is_complete()
            && self
                .pin
                .as_ref()
                .is_some_and(|p| p.matches(&self.key, &self.entry));
        if ok && let Some(digits) = HeldPin::of(&mut self.key, &self.entry) {
            self.digits = Some(digits);
        }
        self.entry.clear();
        if ok {
            self.locked = false;
            self.attempts_left = ATTEMPTS;
        }
        ok
    }

    /// Forgets the PIN and the lock state: the session is wiped, and the
    /// next key loaded sets a new PIN.
    pub fn reset(&mut self) {
        self.pin = None;
        self.digits = None;
        self.entry.clear();
        self.attempts_left = ATTEMPTS;
        self.locked = false;
    }

    // ----- timers -----

    /// Input arrived at `now_ms`: both timers restart.
    pub fn touched(&mut self, now_ms: u64) {
        self.last_input_ms = now_ms;
    }

    /// When the auto-lock fires, in the shell's clock.
    pub fn lock_deadline(&self) -> u64 {
        self.last_input_ms.saturating_add(self.lock_after_ms)
    }

    /// When the auto-wipe fires, if ever.
    pub fn wipe_deadline(&self) -> Option<u64> {
        self.wipe_after_ms
            .map(|ms| self.last_input_ms.saturating_add(ms))
    }

    /// The auto-lock timeout.
    pub fn lock_after_ms(&self) -> u64 {
        self.lock_after_ms
    }

    /// Sets the auto-lock timeout. A wipe timeout shorter than it moves
    /// up to the first wipe option that is not.
    pub fn set_lock_after(&mut self, ms: u64) {
        self.lock_after_ms = ms;
        if let Some(w) = self.wipe_after_ms
            && w < ms
        {
            self.wipe_after_ms = WIPE_OPTIONS
                .iter()
                .find(|o| o.is_none_or(|w| w >= ms))
                .copied()
                .flatten();
        }
    }

    /// The auto-wipe timeout; `None` is never.
    pub fn wipe_after_ms(&self) -> Option<u64> {
        self.wipe_after_ms
    }

    /// Sets the auto-wipe timeout. A lock timeout longer than it moves
    /// down to the last lock option that is not.
    pub fn set_wipe_after(&mut self, ms: Option<u64>) {
        self.wipe_after_ms = ms;
        if let Some(w) = ms
            && self.lock_after_ms > w
        {
            self.lock_after_ms = LOCK_OPTIONS
                .iter()
                .rev()
                .find(|&&l| l <= w)
                .copied()
                .unwrap_or(LOCK_OPTIONS[0]);
        }
    }

    /// Whether the PIN pad is scrambled.
    pub fn scramble_pin(&self) -> bool {
        self.scramble_pin
    }

    /// Turns PIN pad scrambling on or off.
    pub fn set_scramble_pin(&mut self, on: bool) {
        self.scramble_pin = on;
    }

    /// The 32 bytes a master key's curve context is randomized with
    /// (security review M1). Derived from the session key, so it is new
    /// after every rotation; a key that has one blinds every signature
    /// and derivation with a value the machine outside cannot predict.
    pub fn secp_blind(&self) -> [u8; 32] {
        self.key.secp_blind()
    }
}

impl Zeroize for Session {
    fn zeroize(&mut self) {
        self.key.zeroize();
        self.pin = None;
        self.digits = None;
        self.entry.zeroize();
        self.attempts_left = 0;
        self.locked = false;
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Session {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A session whose shell has answered the entropy request, which is
    /// the only kind that takes a PIN.
    fn started() -> Session {
        let mut s = Session::new(b"test");
        s.request_entropy();
        let old = s.entropy(&[9; KEY_LEN]).expect("answered");
        s.reseal(&old);
        s
    }

    /// Rotates the key as a lock does: fresh entropy, and everything
    /// sealed under the old key re-sealed under the new one.
    fn rotate(s: &mut Session, bytes: [u8; KEY_LEN]) {
        s.request_entropy();
        let old = s.entropy(&bytes).expect("answered");
        s.reseal(&old);
    }

    fn pin(s: &str) -> PinEntry {
        let mut p = PinEntry::new();
        for c in s.chars() {
            assert!(p.push(c), "{c}");
        }
        p
    }

    #[test]
    fn entry_accepts_four_to_eight_digits() {
        let mut p = PinEntry::new();
        assert!(!p.push('a'));
        assert!(p.is_empty() && !p.is_complete());
        for c in "123".chars() {
            p.push(c);
        }
        assert!(!p.is_complete());
        p.push('4');
        assert!(p.is_complete());
        for c in "5678".chars() {
            assert!(p.push(c));
        }
        assert!(!p.push('9'), "eight is the most");
        assert_eq!(p.len(), 8);
        p.pop();
        assert_eq!(p.len(), 7);
        assert!(pin("1234").same_as(&pin("1234")));
        assert!(!pin("1234").same_as(&pin("1235")));
        assert!(!pin("1234").same_as(&pin("12345")));
        p.clear();
        assert!(p.is_empty());
    }

    #[test]
    fn pin_is_checked_and_attempts_run_out() {
        let mut s = started();
        assert!(!s.has_pin());
        s.lock();
        assert!(!s.is_locked(), "nothing to lock without a PIN");
        assert!(s.set_pin(&pin("2580")));
        s.lock();
        assert!(s.is_locked());
        for c in "2581".chars() {
            s.entry_push(c);
        }
        assert_eq!(s.submit(), Unlock::Wrong(ATTEMPTS - 1));
        assert!(s.entry().is_empty(), "entry wiped after a try");
        assert!(s.is_locked());
        for c in "2580".chars() {
            s.entry_push(c);
        }
        assert_eq!(s.submit(), Unlock::Unlocked);
        assert!(!s.is_locked());
        assert_eq!(s.attempts_left(), ATTEMPTS, "a right PIN resets the count");
        s.lock();
        for i in 0..ATTEMPTS {
            for c in "0000".chars() {
                s.entry_push(c);
            }
            let r = s.submit();
            if i + 1 == ATTEMPTS {
                assert_eq!(r, Unlock::Wiped);
            } else {
                assert_eq!(r, Unlock::Wrong(ATTEMPTS - i - 1));
            }
        }
        s.reset();
        assert!(!s.has_pin() && !s.is_locked());
    }

    #[test]
    fn a_short_entry_never_unlocks() {
        let mut s = started();
        assert!(s.set_pin(&pin("1234")));
        s.lock();
        s.entry_push('1');
        assert_eq!(s.submit(), Unlock::Wrong(ATTEMPTS - 1));
    }

    #[test]
    fn entropy_rotates_only_when_asked() {
        let mut s = Session::new(b"x");
        assert!(s.is_weak());
        assert!(s.entropy(&[1; 32]).is_none(), "unasked");
        assert!(s.request_entropy());
        assert!(!s.request_entropy(), "one outstanding request");
        let old = s.entropy(&[1; 32]).expect("answered");
        assert!(old.is_weak() && !s.is_weak());
        assert!(!s.entropy_pending());
        assert!(s.set_pin(&pin("4321")));
    }

    #[test]
    fn the_pin_survives_a_lock_and_a_rotated_session_key() {
        let mut s = started();
        assert!(s.set_pin(&pin("4321")));
        assert_eq!(s.with_pin(|d| d.to_vec()), Some(b"4321".to_vec()));
        // A lock, then the key rotates on the shell's fresh entropy, as
        // it does every time the session locks.
        s.lock();
        rotate(&mut s, [2; KEY_LEN]);
        for c in "4320".chars() {
            s.entry_push(c);
        }
        assert_eq!(s.submit(), Unlock::Wrong(ATTEMPTS - 1));
        assert!(s.is_locked());
        for c in "4321".chars() {
            s.entry_push(c);
        }
        assert_eq!(s.submit(), Unlock::Unlocked);
        // The digits behind the PIN came through the rotation too: they
        // are the storage PIN of a key kept on the device.
        assert_eq!(s.with_pin(|d| d.to_vec()), Some(b"4321".to_vec()));
        rotate(&mut s, [3; KEY_LEN]);
        assert_eq!(s.with_pin(|d| d.to_vec()), Some(b"4321".to_vec()));
        s.lock();
        for c in "4321".chars() {
            s.entry_push(c);
        }
        assert_eq!(s.submit(), Unlock::Unlocked);
    }

    #[test]
    fn a_pin_is_not_taken_while_the_session_key_is_weak() {
        let mut s = Session::new(b"test");
        assert!(!s.set_pin(&pin("4321")), "nothing to seal it under");
        assert!(!s.has_pin());
        assert!(s.with_pin(|d| d.to_vec()).is_none());
        s.lock();
        assert!(!s.is_locked());
    }

    #[test]
    fn scramble_seed_follows_the_key_and_the_setting() {
        // §4.3: the shuffle is off by default on every tier.
        let mut a = Session::new(b"m");
        assert!(!a.scramble_pin(), "off by default on Tier C");
        assert_eq!(a.scramble_seed(), None, "no seed while it is off");
        a.set_scramble_pin(true);
        let mut b = Session::new(b"m");
        assert_eq!(b.scramble_seed(), None, "off by default on Tier A");
        b.set_scramble_pin(true);
        assert_eq!(a.scramble_seed(), b.scramble_seed(), "same material");
        a.request_entropy();
        drop(a.entropy(&[7; 32]));
        b.request_entropy();
        drop(b.entropy(&[7; 32]));
        assert_eq!(a.scramble_seed(), b.scramble_seed(), "same entropy");
        b.request_entropy();
        drop(b.entropy(&[8; 32]));
        assert_ne!(a.scramble_seed(), b.scramble_seed());
    }

    #[test]
    fn timers_keep_wipe_at_or_after_lock() {
        let mut s = Session::new(b"m");
        assert_eq!(s.lock_after_ms(), DEFAULT_LOCK_MS);
        assert_eq!(s.wipe_after_ms(), Some(DEFAULT_WIPE_MS));
        s.set_lock_after(900_000);
        assert_eq!(s.wipe_after_ms(), Some(1_800_000), "wipe moved up");
        s.set_wipe_after(Some(300_000));
        assert_eq!(s.lock_after_ms(), 300_000, "lock moved down");
        s.set_wipe_after(None);
        s.set_lock_after(900_000);
        assert_eq!(s.wipe_after_ms(), None);
        s.touched(1_000);
        assert_eq!(s.lock_deadline(), 901_000);
        assert_eq!(s.wipe_deadline(), None);
        s.set_wipe_after(Some(3_600_000));
        assert_eq!(s.wipe_deadline(), Some(3_601_000));
    }
}
