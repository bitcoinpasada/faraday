//! The key kept on the device: the blob's format and its cryptography
//! (`docs/PLANNING.md` §6, §15 items 32 and 9). The exchange with the
//! element and the PIN pad that drive it are the app's; this crate is
//! what the bytes are (§16.138).
//!
//! A person on a Tier B device may keep their keys on it. No seed is
//! ever stored: each key's words are, and only as ciphertext. The key
//! that decrypts them is
//!
//! ```text
//! stretched = Argon2id(pin, salt_pin)
//! challenge = HMAC-SHA256(key = stretched, message = salt_se)
//! mac       = HMAC_SE(challenge)
//! KEK       = HMAC-SHA256(key = stretched, message = mac)
//! ```
//!
//! where `HMAC_SE` is a secure element's answer to `challenge` — in
//! OpenSigner, the shell's answer to `Command::SecureMac`: HMAC-SHA256
//! under a key
//! that never leaves the element and needs the person's authentication
//! for every use. Neither half is enough. Argon2id makes a guess cost
//! memory and time, and the element makes every guess cost one
//! authentication on that one chip.
//!
//! What the element is asked to authenticate is derived from the PIN
//! being tried, so its answer belongs to that one candidate. An
//! attacker who reads a tag out of the app's memory while the person
//! unlocks the device has the tag for the PIN the person typed, which
//! is the PIN that already opened the blob; it is worth nothing against
//! any other candidate, and every other candidate still costs one
//! authentication on the chip. Eight wrong PINs and the app asks the
//! shell to forget the blob and the element's key with it, which makes
//! every copy of the bytes permanently useless.
//!
//! The blob holds four records. The keys record holds every kept key —
//! room for [`MAX_KEPT`] of them, and the same size with one in it as
//! with a hundred, so the stored bytes do not say how many keys there
//! are — and opens under the storage PIN. The wallets record holds the
//! wallets in use, which are public data kept under the same protection
//! and under the same key, and is the same size however many there are.
//! The notes record holds the notes and the recovery sheets the person
//! said to keep, under that same key. The duress record opens under the
//! duress PIN when one is set, and under nothing at all when none is,
//! because it is then random bytes under a random key. All four look
//! random, so the stored bytes do not say whether a duress PIN exists,
//! and a person forced to unlock can open a device that erases itself.
//!
//! The keys, the wallets and the notes on the device follow the keys,
//! the wallets and the notes in memory: a key added, forgotten or
//! backed up, a wallet used or forgotten, or an item kept or dropped,
//! while keys are kept rewrites those three records under the key that
//! opened them ([`rewrite`]), with no word to the element, and
//! forgetting the last key forgets the blob and the rest with it.
//!
//! Layout, all little-endian, 77039 bytes:
//!
//! ```text
//!     0    version, 1
//!     1    Argon2 memory cost, u32, in KiB
//!     5    Argon2 passes, u32
//!     9    Argon2 lanes, u32
//!    13    salt_pin, 32 bytes
//!    45    salt_se, 32 bytes           -- 1..77 is the associated data
//!    77    attempts used, u8
//!    78    keys record:    nonce 24, ciphertext 5102, tag 16
//!  5220    duress record:  nonce 24, ciphertext 52, tag 16
//!  5312    wallets record: nonce 24, ciphertext 36866, tag 16
//! 42218    notes record:   nonce 24, ciphertext 34818, tag 16
//! ```
//!
//! The wallets record's plaintext is a kind byte and
//! [`crate::wallets`]'s body: a count and sixteen slots of 2,304
//! bytes, each one wallet's text zero-padded. The notes record's is a
//! kind byte and [`crate::notes`]'s body: a count and eight slots
//! of 4,352 bytes, each one item's kind, length and payload. Those two
//! modules hold the text and the item side of it, because this file may
//! hold neither; what crosses between them is a fixed-length byte
//! array.
//!
//! A version-4 blob — the layout without the notes record, which is
//! every blob written before this build — is read rather than refused.
//! Its header and its three records are at these offsets and its bytes
//! are untouched by the change, so it opens under the same PIN and the
//! same element answer; it simply has no notes record, and [`open`]
//! answers with no items. The first [`rewrite`] after that grows the
//! buffer, writes the notes record and sets the version byte to 5.
//! Version 3 and below stay refused, as §16.53 has it: those layouts
//! hold no wallets record and version 1's element exchange was the one
//! a captured tag broke.
//!
//! The version byte is outside the associated data for exactly that
//! reason. Every record is bound to the header, and the duress record
//! can only be written by the duress PIN's own element answer, which an
//! upgrade does not have; a version byte inside the associated data
//! would therefore make an upgraded blob's duress PIN stop working.
//! What the byte is for is the layout, and the layout is checked
//! against the blob's length by [`header`], so a flipped byte opens
//! nothing. Byte 0 of the associated data is the constant
//! [`AAD_VERSION`], which is the value a version-4 blob was sealed
//! under, so those records verify unchanged.
//!
//! `salt_se` is fixed for the life of a blob, and it is not what the
//! element sees: the challenge is, and that is a different value for
//! every PIN tried against these bytes.
//!
//! The associated data stops short of the attempt counter on purpose:
//! the counter is bumped after a wrong PIN by a caller that, by
//! definition, cannot re-encrypt the records, and set back to zero by
//! the right one. An attacker with the bytes can set it back to zero
//! too; what that buys is more attempts, and each one still costs an
//! authentication on the element that wrote them. Every other byte of
//! the header is authenticated, so a blob whose parameters or salts
//! were changed opens nothing.
//!
//! The keys record's plaintext is a kind byte, a count, and [`MAX_KEPT`]
//! slots of 51 bytes each — the 50 bytes of a [`MnemonicBytes`] and the
//! backup-verified flag — zero where no key is. The duress record's is a
//! kind byte and one such slot. A passphrase is never among them: a
//! passphrase-protected wallet stays behind what the person types each
//! session.
//!
//! A slot says what it holds by its word count, which is at byte 48 and
//! is 12 to 24 for words (`docs/PLANNING.md` §16.107 rule 6). A count of
//! zero marks a key with no words instead: byte 49, where a mnemonic's
//! wordlist tag sits, is the seed's length — one of
//! [`MASTER_SECRET_LENGTHS`] — and the seed itself is the first bytes of
//! the slot, where the word indices would be. Byte 47 says which kind of
//! key it is, 0 for SLIP-39 shares and 1 for a codex32 string
//! (§16.109 rule 1); it is past the longest seed a slot holds, and it is
//! zero in every slot written before codex32 existed, which is what a
//! SLIP-39 key reads as. Every record keeps its size, so nothing in a
//! blob this build wrote can be read the wrong way: every occupied slot
//! it wrote has 12 to 24 words.
//!
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text.

#![no_std]

extern crate alloc;

pub mod names;
pub mod notes;
pub mod wallets;

use alloc::vec;
use alloc::vec::Vec;

use argon2::Params;
use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{AeadInOut, KeyInit, Tag, XChaCha20Poly1305, XNonce};
use hmac::{Hmac, Mac};
use osk_crypto::{MAX_MNEMONIC_WORDS, MnemonicBytes, SealedBytes, Secret, SeedBytes, Zeroize};
use sha2::{Sha256, Sha512};

use osk_backup::{Cost, DEVICE_PARAMS};
use zeroize::Zeroizing;

use crate::notes::KeptNotes;

/// Wrong PINs the app allows before it asks the shell to forget the
/// stored key, which deletes the element's key with the bytes. The
/// element itself counts nothing about this PIN: what it does is make
/// every attempt cost one authentication.
pub const KEEP_ATTEMPTS: u8 = 8;

/// The blob's format version, which is what [`write`] and [`rewrite`]
/// put in byte 0. Version 1 asked the element to authenticate a value
/// fixed in the header, so one captured tag made every later guess
/// offline; version 2 held one key; version 3 held every key but no
/// wallets. None of the three is a blob this build reads: [`header`]
/// refuses them, and a shell holding one is a shell holding no blob this
/// app can open, which is what the app already does with bytes it cannot
/// read.
pub const VERSION: u8 = 5;

/// The version before it: the same header and the same three first
/// records, with no notes record after them. [`header`] and [`open`]
/// read one, so a device that was keeping keys yesterday still has them
/// today, and the next [`rewrite`] makes it a [`VERSION`] blob.
pub const VERSION_V4: u8 = 4;

/// The version byte the associated data carries, whichever version the
/// blob is. It is [`VERSION_V4`] because that is what every record
/// already on a device was sealed under, and a record cannot be
/// re-sealed by an upgrade: the duress record's key is the duress PIN's
/// own, which no upgrade holds.
const AAD_VERSION: u8 = VERSION_V4;

/// Bytes of a [`MnemonicBytes`] in its sealable form.
const MNEMONIC_LEN: usize = MAX_MNEMONIC_WORDS * 2 + 2;
/// One kept key inside a record: the words and the backup flag.
const KEPT_LEN: usize = MNEMONIC_LEN + 1;
/// The seed lengths a tagged slot carries in place of words: SLIP-39's
/// two master-secret sizes and the BIP 93 sizes that fit the room a
/// slot has beside the word count, which is every one of them but 512
/// bits (`docs/PLANNING.md` §16.107 rule 6, §16.109 rule 1).
const MASTER_SECRET_LENGTHS: [usize; 5] = [16, 20, 24, 28, 32];

/// Whether a key with no words can be kept: its seed is one of the
/// lengths a slot holds. A 64-byte codex32 seed is not, and such a key
/// is not offered to the device.
pub fn keeps_secret(len: usize) -> bool {
    MASTER_SECRET_LENGTHS.contains(&len)
}
/// Keys the blob has room for. The record is the same size with one key
/// in it and with a hundred, so the stored bytes say nothing about how
/// many there are, and a hundred is more than a person will load.
pub const MAX_KEPT: usize = 100;
/// The keys record's plaintext: kind, count, then every slot.
const KEYS_PLAIN_LEN: usize = 2 + MAX_KEPT * KEPT_LEN;
/// The duress record's plaintext: kind, then the shape of one key.
const DURESS_PLAIN_LEN: usize = 1 + KEPT_LEN;
/// The wallets record's plaintext: kind, then the wallets body.
const WALLETS_PLAIN_LEN: usize = 1 + crate::wallets::BODY_LEN;
/// The notes record's plaintext: kind, then the notes body.
const NOTES_PLAIN_LEN: usize = 1 + crate::notes::BODY_LEN;
/// XChaCha20-Poly1305's nonce.
const NONCE_LEN: usize = 24;
/// Poly1305's tag.
const TAG_LEN: usize = 16;
/// The keys record: nonce, ciphertext, tag.
const KEYS_RECORD_LEN: usize = NONCE_LEN + KEYS_PLAIN_LEN + TAG_LEN;
/// The duress record: nonce, ciphertext, tag.
const DURESS_RECORD_LEN: usize = NONCE_LEN + DURESS_PLAIN_LEN + TAG_LEN;
/// The wallets record: nonce, ciphertext, tag.
const WALLETS_RECORD_LEN: usize = NONCE_LEN + WALLETS_PLAIN_LEN + TAG_LEN;
/// The notes record: nonce, ciphertext, tag.
const NOTES_RECORD_LEN: usize = NONCE_LEN + NOTES_PLAIN_LEN + TAG_LEN;
/// The part of the header both records are bound to.
const HEADER_LEN: usize = 1 + 4 + 4 + 4 + 32 + 32;
/// Where the attempt counter sits.
const ATTEMPTS_AT: usize = HEADER_LEN;
/// Where the keys record starts.
const KEY_AT: usize = ATTEMPTS_AT + 1;
/// Where the duress record starts.
const DURESS_AT: usize = KEY_AT + KEYS_RECORD_LEN;
/// Where the wallets record starts.
const WALLETS_AT: usize = DURESS_AT + DURESS_RECORD_LEN;
/// Where the notes record starts, which is where a version-4 blob ends.
const NOTES_AT: usize = WALLETS_AT + WALLETS_RECORD_LEN;
/// The whole blob.
pub const BLOB_LEN: usize = NOTES_AT + NOTES_RECORD_LEN;
/// The whole of a [`VERSION_V4`] blob, which is this one without its
/// notes record.
pub const BLOB_LEN_V4: usize = NOTES_AT;

/// The kind byte of the key record.
const KIND_KEY: u8 = 1;
/// The kind byte of the duress record.
const KIND_DURESS: u8 = 2;
/// The kind byte of the wallets record.
const KIND_WALLETS: u8 = 3;
/// The kind byte of the notes record.
const KIND_NOTES: u8 = 4;

/// Domain separation for the bytes drawn from a caller's seed.
const SALT_PIN_LABEL: u8 = 1;
const SALT_SE_LABEL: u8 = 2;
const KEY_NONCE_LABEL: u8 = 3;
const DURESS_NONCE_LABEL: u8 = 4;
const DECOY_LABEL: u8 = 5;
const WALLETS_NONCE_LABEL: u8 = 6;
const NOTES_NONCE_LABEL: u8 = 7;

/// A blob's header: what a reader needs before it can ask the secure
/// element anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// The Argon2id cost this blob was written at.
    pub cost: Cost,
    /// The Argon2id salt over the PIN.
    pub salt_pin: [u8; 32],
    /// What the secure element is asked to authenticate.
    pub salt_se: [u8; 32],
    /// Wrong PINs entered against this blob so far.
    pub attempts: u8,
}

impl Header {
    /// Attempts left before the stored key is forgotten.
    pub fn attempts_left(&self) -> u8 {
        KEEP_ATTEMPTS.saturating_sub(self.attempts)
    }
}

/// What one kept key is: the words it was typed as, or the master
/// secret of a SLIP-39 key, which has none.
pub enum KeptSecret {
    /// BIP-39 words, as they are sealed in memory.
    Words(MnemonicBytes),
    /// The seed of a key with no words: a SLIP-39 master secret, 16 or
    /// 32 bytes, or a codex32 seed of any BIP 93 length the slot holds.
    MasterSecret {
        /// The seed itself.
        secret: SeedBytes,
        /// Whether it was read from a codex32 string rather than from
        /// SLIP-39 shares, which the byte before the tag records so that
        /// a 16- or 32-byte seed comes back as the kind it was.
        codex32: bool,
    },
}

impl Zeroize for KeptSecret {
    fn zeroize(&mut self) {
        match self {
            KeptSecret::Words(m) => m.zeroize(),
            KeptSecret::MasterSecret { secret, codex32 } => {
                secret.zeroize();
                *codex32 = false;
            }
        }
    }
}

/// One kept key: what it is made of and the backup flag.
pub struct Kept {
    /// The words or the master secret.
    pub secret: KeptSecret,
    /// Whether the backup quiz had been passed when the key was kept.
    pub backup_verified: bool,
}

impl Kept {
    /// An empty slot.
    fn empty() -> Self {
        Kept {
            secret: KeptSecret::Words(MnemonicBytes::from_bytes(
                &<MnemonicBytes as SealedBytes>::ZEROED,
            )),
            backup_verified: false,
        }
    }

    /// The words this key is, where it has any.
    pub fn mnemonic(&self) -> Option<&MnemonicBytes> {
        match &self.secret {
            KeptSecret::Words(m) => Some(m),
            KeptSecret::MasterSecret { .. } => None,
        }
    }

    /// Writes the slot's 51 bytes.
    fn write(&self, out: &mut [u8]) {
        match &self.secret {
            KeptSecret::Words(m) => {
                let mut bytes = <MnemonicBytes as SealedBytes>::ZEROED;
                m.write_bytes(&mut bytes);
                out[..MNEMONIC_LEN].copy_from_slice(&bytes);
                bytes.zeroize();
            }
            KeptSecret::MasterSecret { secret, codex32 } => {
                let bytes = secret.as_bytes();
                out[..bytes.len()].copy_from_slice(bytes);
                out[MNEMONIC_LEN - 3] = u8::from(*codex32);
                out[MNEMONIC_LEN - 2] = 0;
                out[MNEMONIC_LEN - 1] = bytes.len() as u8;
            }
        }
        out[MNEMONIC_LEN] = u8::from(self.backup_verified);
    }

    /// Reads a slot back. `None` where the bytes are neither a mnemonic
    /// nor a master secret of a length SLIP-39 works in.
    fn read(slot: &[u8]) -> Option<Self> {
        let backup_verified = slot[MNEMONIC_LEN] == 1;
        if slot[MNEMONIC_LEN - 2] == 0 {
            let len = usize::from(slot[MNEMONIC_LEN - 1]);
            if !keeps_secret(len) {
                return None;
            }
            let secret = SeedBytes::new(&slot[..len])?;
            return Some(Kept {
                secret: KeptSecret::MasterSecret {
                    secret,
                    codex32: slot[MNEMONIC_LEN - 3] == 1,
                },
                backup_verified,
            });
        }
        let mut bytes = <MnemonicBytes as SealedBytes>::ZEROED;
        bytes.copy_from_slice(&slot[..MNEMONIC_LEN]);
        let mnemonic = MnemonicBytes::from_bytes(&bytes);
        bytes.zeroize();
        Some(Kept {
            secret: KeptSecret::Words(mnemonic),
            backup_verified,
        })
    }
}

impl Zeroize for Kept {
    fn zeroize(&mut self) {
        self.secret.zeroize();
        self.backup_verified = false;
    }
}

impl Drop for Kept {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Every key the device keeps, in the order they load back. Room for
/// [`MAX_KEPT`]; `push` refuses the rest.
pub struct KeptKeys {
    count: u8,
    keys: [Kept; MAX_KEPT],
}

impl KeptKeys {
    /// No keys.
    pub fn new() -> Self {
        KeptKeys {
            count: 0,
            keys: core::array::from_fn(|_| Kept::empty()),
        }
    }

    /// Adds a key; `false`, and nothing added, when every slot is taken.
    pub fn push(&mut self, kept: Kept) -> bool {
        let n = usize::from(self.count);
        if n >= MAX_KEPT {
            return false;
        }
        self.keys[n] = kept;
        self.count += 1;
        true
    }

    /// How many keys.
    pub fn len(&self) -> usize {
        usize::from(self.count)
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The keys, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Kept> {
        self.keys[..self.len()].iter()
    }
}

impl Default for KeptKeys {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for KeptKeys {
    fn zeroize(&mut self) {
        for k in &mut self.keys {
            k.zeroize();
        }
        self.count = 0;
    }
}

impl Drop for KeptKeys {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// The key the keys record is sealed under, kept for the session that
/// wrote or opened the blob so that [`rewrite`] can follow the keys in
/// memory without asking the element again. Sealable, so the session
/// holds it the way it holds every other secret.
pub struct Kek(Secret<[u8; 32]>);

impl Zeroize for Kek {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl SealedBytes for Kek {
    type Bytes = [u8; 32];
    const ZEROED: [u8; 32] = [0; 32];

    fn write_bytes(&self, out: &mut [u8; 32]) {
        out.copy_from_slice(self.0.expose());
    }

    fn from_bytes(bytes: &[u8; 32]) -> Self {
        Kek(Secret::new(*bytes))
    }
}

/// What a PIN opened.
// The keys are five kilobytes on the stack, moved once to the caller
// that loads them; a heap copy would be one more place for them to be.
#[allow(clippy::large_enum_variant)]
pub enum Opened {
    /// The storage PIN: here are the keys, the wallets record's body —
    /// public data, which [`crate::wallets`] reads — the notes and
    /// sheets the notes record held, and what all three are sealed
    /// under. A version-4 blob has no notes record, so its items are
    /// none.
    Keys(KeptKeys, Vec<u8>, KeptNotes, Kek),
    /// The duress PIN: the caller erases everything and shows an empty
    /// app.
    Duress,
    /// Neither record opened.
    Wrong,
}

/// Whether an Argon2id cost is one this build would have written:
/// between what the `argon2` crate accepts at the low end and
/// [`DEVICE_PARAMS`] at the high end, each of the three separately. The
/// upper bound is what keeps the blob from choosing the work: the header
/// is associated data, so a changed cost can only fail authentication,
/// but the memory is allocated and the passes are run before that is
/// known. A blob asking for 16 GiB aborts on the allocation and one
/// asking for hundreds of passes never returns.
fn cost_usable(cost: &Cost) -> bool {
    (Params::MIN_M_COST..=DEVICE_PARAMS.memory_kib).contains(&cost.memory_kib)
        && (Params::MIN_T_COST..=DEVICE_PARAMS.passes).contains(&cost.passes)
        && (Params::MIN_P_COST..=DEVICE_PARAMS.lanes).contains(&cost.lanes)
}

/// Whether `blob`'s length and version byte are one of the two layouts
/// this build reads. The version says where the records are and the
/// length has to agree with it, which is what makes the version byte
/// worth nothing to an attacker who edits it.
fn laid_out(blob: &[u8]) -> bool {
    match blob.first() {
        Some(&VERSION) => blob.len() == BLOB_LEN,
        Some(&VERSION_V4) => blob.len() == BLOB_LEN_V4,
        _ => false,
    }
}

/// Whether `blob` carries a notes record at all.
fn has_notes_record(blob: &[u8]) -> bool {
    blob.len() == BLOB_LEN && blob.first() == Some(&VERSION)
}

/// The header of `blob`, or `None` when the bytes are not a blob this
/// build wrote.
pub fn header(blob: &[u8]) -> Option<Header> {
    if !laid_out(blob) {
        return None;
    }
    let u32_at = |i: usize| u32::from_le_bytes([blob[i], blob[i + 1], blob[i + 2], blob[i + 3]]);
    let mut salt_pin = [0u8; 32];
    let mut salt_se = [0u8; 32];
    salt_pin.copy_from_slice(&blob[13..45]);
    salt_se.copy_from_slice(&blob[45..77]);
    let cost = Cost {
        memory_kib: u32_at(1),
        passes: u32_at(5),
        lanes: u32_at(9),
    };
    if !cost_usable(&cost) {
        return None;
    }
    Some(Header {
        cost,
        salt_pin,
        salt_se,
        attempts: blob[ATTEMPTS_AT],
    })
}

/// A header for a fresh blob at `cost`, its two salts drawn from `seed`.
pub fn new_header(cost: Cost, seed: &[u8; 32]) -> Header {
    let mut salt_pin = [0u8; 32];
    let mut salt_se = [0u8; 32];
    draw(seed, SALT_PIN_LABEL, &mut salt_pin);
    draw(seed, SALT_SE_LABEL, &mut salt_se);
    Header {
        cost,
        salt_pin,
        salt_se,
        attempts: 0,
    }
}

/// Writes a blob: `keys` under the storage PIN `guess` was made from,
/// and a duress record that is random bytes under a random key until a
/// duress PIN replaces it through [`set_duress`]. `mac` is the secure
/// element's answer for `guess`'s challenge, and `seed` supplies the
/// nonces and the decoy. The key the keys record is sealed under comes
/// back with the blob, for [`rewrite`].
pub fn write(
    header: &Header,
    mac: &[u8; 32],
    guess: &Guess,
    keys: &KeptKeys,
    wallets: &[u8],
    notes: &KeptNotes,
    seed: &[u8; 32],
) -> (Vec<u8>, Kek) {
    let mut blob = vec![0u8; BLOB_LEN];
    blob[0] = VERSION;
    blob[1..5].copy_from_slice(&header.cost.memory_kib.to_le_bytes());
    blob[5..9].copy_from_slice(&header.cost.passes.to_le_bytes());
    blob[9..13].copy_from_slice(&header.cost.lanes.to_le_bytes());
    blob[13..45].copy_from_slice(&header.salt_pin);
    blob[45..77].copy_from_slice(&header.salt_se);
    blob[ATTEMPTS_AT] = header.attempts;

    let kek = Kek(kek(guess, mac));
    seal_keys(&mut blob, &kek, keys, seed);
    seal_wallets(&mut blob, &kek, wallets, seed);
    seal_notes(&mut blob, &kek, notes, seed);
    write_duress(&mut blob, mac, None, seed);
    (blob, kek)
}

/// Replaces the keys, wallets and notes records of an existing blob
/// under the key that opened it, leaving the header and the duress
/// record alone: what is on the device follows what is in memory with
/// no word to the element. `seed` must be fresh for every call — the
/// same key under the same nonce twice is the one thing this cipher does
/// not survive — which the caller's per-write derivation from the
/// session key makes it.
///
/// A version-4 blob grows here: the buffer gains a notes record and the
/// version byte becomes [`VERSION`]. The duress record is not touched
/// and does not have to be, because the version byte is outside the
/// associated data.
pub fn rewrite(
    blob: &mut Vec<u8>,
    kek: &Kek,
    keys: &KeptKeys,
    wallets: &[u8],
    notes: &KeptNotes,
    seed: &[u8; 32],
) -> bool {
    if header(blob).is_none() {
        return false;
    }
    if blob.len() == BLOB_LEN_V4 {
        blob.resize(BLOB_LEN, 0);
        blob[0] = VERSION;
    }
    seal_keys(blob, kek, keys, seed);
    seal_wallets(blob, kek, wallets, seed);
    seal_notes(blob, kek, notes, seed);
    true
}

/// Seals `keys` into the keys record, under a nonce drawn from `seed`.
fn seal_keys(blob: &mut [u8], kek: &Kek, keys: &KeptKeys, seed: &[u8; 32]) {
    let mut plain = Secret::new(keys_bytes(keys));
    let mut nonce = [0u8; NONCE_LEN];
    draw(seed, KEY_NONCE_LABEL, &mut nonce);
    seal(blob, KEY_AT, kek.0.expose(), &nonce, plain.expose_mut());
    plain.zeroize();
}

/// Seals the wallets body into the wallets record, under the same key
/// the keys record uses and a nonce of its own. The body is public data,
/// so it is a plain buffer; a body of the wrong length is padded or cut
/// to the record's size, which is fixed.
fn seal_wallets(blob: &mut [u8], kek: &Kek, wallets: &[u8], seed: &[u8; 32]) {
    let mut plain = vec![0u8; WALLETS_PLAIN_LEN];
    plain[0] = KIND_WALLETS;
    let n = wallets.len().min(WALLETS_PLAIN_LEN - 1);
    plain[1..1 + n].copy_from_slice(&wallets[..n]);
    let mut nonce = [0u8; NONCE_LEN];
    draw(seed, WALLETS_NONCE_LABEL, &mut nonce);
    seal(blob, WALLETS_AT, kek.0.expose(), &nonce, &mut plain);
}

/// Seals the notes body into the notes record, under the same key the
/// keys record uses and a nonce of its own. An item is what a person
/// wrote, so the plaintext is built in a buffer that zeroizes.
fn seal_notes(blob: &mut [u8], kek: &Kek, notes: &KeptNotes, seed: &[u8; 32]) {
    let mut plain: Zeroizing<Vec<u8>> = Zeroizing::new(vec![0u8; NOTES_PLAIN_LEN]);
    plain[0] = KIND_NOTES;
    notes.write_body(&mut plain[1..]);
    let mut nonce = [0u8; NONCE_LEN];
    draw(seed, NOTES_NONCE_LABEL, &mut nonce);
    seal(blob, NOTES_AT, kek.0.expose(), &nonce, &mut plain);
}

/// Replaces the duress record of an existing blob, leaving the key
/// record alone: setting a duress PIN needs one secure-element tag for
/// that PIN's own challenge and not the storage PIN.
pub fn set_duress(
    blob: &mut [u8],
    mac: &[u8; 32],
    duress: Option<&Guess>,
    seed: &[u8; 32],
) -> bool {
    if header(blob).is_none() {
        return false;
    }
    write_duress(blob, mac, duress, seed);
    true
}

fn write_duress(blob: &mut [u8], mac: &[u8; 32], duress: Option<&Guess>, seed: &[u8; 32]) {
    let mut nonce = [0u8; NONCE_LEN];
    draw(seed, DURESS_NONCE_LABEL, &mut nonce);
    match duress {
        // The marker, under the key the duress PIN and the element's
        // answer to that PIN's challenge derive.
        Some(guess) => {
            let kek = kek(guess, mac);
            let mut plain = Secret::new(duress_bytes());
            seal(blob, DURESS_AT, kek.expose(), &nonce, plain.expose_mut());
            plain.zeroize();
        }
        // No duress PIN: a record of the same shape that nothing opens,
        // so the stored bytes never say the two apart. Its key and its
        // plaintext are both drawn from the seed and neither is kept.
        None => {
            let mut material = Zeroizing::new([0u8; 32 + DURESS_PLAIN_LEN]);
            draw(seed, DECOY_LABEL, material.as_mut());
            let mut decoy_key = [0u8; 32];
            let mut plain = [0u8; DURESS_PLAIN_LEN];
            decoy_key.copy_from_slice(&material[..32]);
            plain.copy_from_slice(&material[32..]);
            seal(blob, DURESS_AT, &decoy_key, &nonce, &mut plain);
            decoy_key.zeroize();
            plain.zeroize();
        }
    }
}

/// Tries one typed PIN against both records of `blob`, with `mac` the
/// secure element's answer for that PIN's challenge. A tag made for
/// another PIN's challenge opens neither record.
pub fn open(blob: &[u8], mac: &[u8; 32], guess: &Guess) -> Opened {
    if header(blob).is_none() {
        return Opened::Wrong;
    }
    let kek = Kek(kek(guess, mac));
    if let Some(plain) = unseal::<KEYS_PLAIN_LEN>(blob, KEY_AT, kek.0.expose())
        && plain.expose()[0] == KIND_KEY
        && let Some(keys) = keys_of(plain.expose())
    {
        // The wallets are public data and not what the PIN is for: a
        // record that does not open leaves the keys open and the wallet
        // list empty.
        let wallets = unseal_public(blob, WALLETS_AT, WALLETS_PLAIN_LEN, kek.0.expose())
            .filter(|plain| plain[0] == KIND_WALLETS)
            .map_or_else(Vec::new, |plain| plain[1..].to_vec());
        // A version-4 blob has no notes record. One that does and does
        // not open leaves the keys open and no items, for the reason
        // the wallets have.
        let notes = if has_notes_record(blob) {
            unseal_public(blob, NOTES_AT, NOTES_PLAIN_LEN, kek.0.expose())
                .map(Zeroizing::new)
                .filter(|plain| plain[0] == KIND_NOTES)
                .map_or_else(KeptNotes::new, |plain| crate::notes::items(&plain[1..]))
        } else {
            KeptNotes::new()
        };
        return Opened::Keys(keys, wallets, notes, kek);
    }
    if let Some(plain) = unseal::<DURESS_PLAIN_LEN>(blob, DURESS_AT, kek.0.expose())
        && plain.expose()[0] == KIND_DURESS
    {
        return Opened::Duress;
    }
    Opened::Wrong
}

/// Counts one wrong PIN against `blob` and answers how many are left.
/// The record bytes are untouched, so the blob still opens under the
/// right PIN.
pub fn count_attempt(blob: &mut [u8]) -> u8 {
    if !laid_out(blob) {
        return 0;
    }
    blob[ATTEMPTS_AT] = blob[ATTEMPTS_AT].saturating_add(1);
    KEEP_ATTEMPTS.saturating_sub(blob[ATTEMPTS_AT])
}

/// A right PIN ends the run of wrong ones: the count is of PINs typed
/// since the key last opened, not since it was kept. `true` when the
/// count was above zero and the blob should be stored again.
pub fn reset_attempts(blob: &mut [u8]) -> bool {
    if !laid_out(blob) || blob[ATTEMPTS_AT] == 0 {
        return false;
    }
    blob[ATTEMPTS_AT] = 0;
    true
}

/// One typed PIN, stretched. Argon2id runs once, where this is made:
/// the challenge the secure element authenticates and the key that
/// opens a record both come from the same stretched bytes, so one guess
/// costs one Argon2id and one authentication however many records it is
/// tried against. Dropping it zeroizes both.
pub struct Guess {
    stretched: Secret<[u8; 32]>,
    challenge: [u8; 32],
}

impl Guess {
    /// What the element is asked to authenticate for this guess, which
    /// is the salt of the `SecureMac` command the shell answers. No
    /// other PIN produces it, so the answer belongs to this guess.
    pub fn challenge(&self) -> &[u8; 32] {
        &self.challenge
    }
}

impl Zeroize for Guess {
    fn zeroize(&mut self) {
        self.stretched.zeroize();
        self.challenge.zeroize();
    }
}

impl Drop for Guess {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Stretches `pin` under the blob's parameters and derives the
/// challenge for `header.salt_se`. `None` when the Argon2 parameters in
/// the header are not usable.
pub fn challenge(header: &Header, pin: &[u8]) -> Option<Guess> {
    let stretched = stretch(header, pin)?;
    let mut hmac =
        Hmac::<Sha256>::new_from_slice(stretched.expose()).expect("HMAC accepts any key length");
    hmac.update(&header.salt_se);
    Some(Guess {
        stretched,
        challenge: hmac.finalize().into_bytes().into(),
    })
}

/// `HMAC-SHA256(key = stretched, message = mac)`: the key a record is
/// sealed under, given what the element answered this guess with.
fn kek(guess: &Guess, mac: &[u8; 32]) -> Secret<[u8; 32]> {
    let mut hmac = Hmac::<Sha256>::new_from_slice(guess.stretched.expose())
        .expect("HMAC accepts any key length");
    hmac.update(mac);
    Secret::new(hmac.finalize().into_bytes().into())
}

/// `Argon2id(pin, salt_pin)` at the blob's cost
/// ([`osk_backup::argon2id`]).
fn stretch(header: &Header, pin: &[u8]) -> Option<Secret<[u8; 32]>> {
    osk_backup::argon2id(&header.cost, pin, &header.salt_pin).ok()
}

/// The keys record's plaintext: the kind, the count and every slot.
fn keys_bytes(keys: &KeptKeys) -> [u8; KEYS_PLAIN_LEN] {
    let mut out = [0u8; KEYS_PLAIN_LEN];
    out[0] = KIND_KEY;
    out[1] = keys.count;
    for (i, k) in keys.iter().enumerate() {
        let at = 2 + i * KEPT_LEN;
        k.write(&mut out[at..at + KEPT_LEN]);
    }
    out
}

/// The reverse, for a keys record that authenticated. `None` when the
/// count is more than the record has slots.
fn keys_of(plain: &[u8; KEYS_PLAIN_LEN]) -> Option<KeptKeys> {
    let count = usize::from(plain[1]);
    if count > MAX_KEPT {
        return None;
    }
    let mut keys = KeptKeys::new();
    for i in 0..count {
        let at = 2 + i * KEPT_LEN;
        let Some(kept) = Kept::read(&plain[at..at + KEPT_LEN]) else {
            continue;
        };
        keys.push(kept);
    }
    Some(keys)
}

/// The duress record's plaintext: the kind and an empty slot.
fn duress_bytes() -> [u8; DURESS_PLAIN_LEN] {
    let mut out = [0u8; DURESS_PLAIN_LEN];
    out[0] = KIND_DURESS;
    out
}

/// Encrypts `plain` into the record at `at`, with the header as
/// associated data. `plain` is left as ciphertext and the caller
/// zeroizes it.
fn seal(blob: &mut [u8], at: usize, key: &[u8; 32], nonce: &[u8; NONCE_LEN], plain: &mut [u8]) {
    let cipher = XChaCha20Poly1305::new_from_slice(key).expect("32-byte key");
    let aad = header_bytes(blob);
    let tag = cipher
        .encrypt_inout_detached(&XNonce::from(*nonce), &aad, InOutBuf::from(&mut *plain))
        .expect("a record is far below the AEAD's length limit");
    let n = plain.len();
    blob[at..at + NONCE_LEN].copy_from_slice(nonce);
    blob[at + NONCE_LEN..at + NONCE_LEN + n].copy_from_slice(plain);
    blob[at + NONCE_LEN + n..at + NONCE_LEN + n + TAG_LEN].copy_from_slice(&tag);
}

/// Decrypts the `N`-byte record at `at`; `None` when the tag does not
/// verify.
fn unseal<const N: usize>(blob: &[u8], at: usize, key: &[u8; 32]) -> Option<Secret<[u8; N]>> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).expect("32-byte key");
    let aad = header_bytes(blob);
    let mut plain = Secret::new([0u8; N]);
    plain
        .expose_mut()
        .copy_from_slice(&blob[at + NONCE_LEN..at + NONCE_LEN + N]);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&blob[at..at + NONCE_LEN]);
    let mut tag = [0u8; TAG_LEN];
    tag.copy_from_slice(&blob[at + NONCE_LEN + N..at + NONCE_LEN + N + TAG_LEN]);
    cipher
        .decrypt_inout_detached(
            &XNonce::from(nonce),
            &aad,
            InOutBuf::from(plain.expose_mut().as_mut_slice()),
            &Tag::from(tag),
        )
        .ok()
        .map(|()| plain)
}

/// The same, for a record that is not secret: the plaintext comes back
/// on the heap rather than as a fixed array on the stack, because the
/// wallets record is tens of kilobytes and holds only public data.
fn unseal_public(blob: &[u8], at: usize, len: usize, key: &[u8; 32]) -> Option<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).expect("32-byte key");
    let aad = header_bytes(blob);
    let mut plain = blob.get(at + NONCE_LEN..at + NONCE_LEN + len)?.to_vec();
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&blob[at..at + NONCE_LEN]);
    let mut tag = [0u8; TAG_LEN];
    tag.copy_from_slice(&blob[at + NONCE_LEN + len..at + NONCE_LEN + len + TAG_LEN]);
    cipher
        .decrypt_inout_detached(
            &XNonce::from(nonce),
            &aad,
            InOutBuf::from(plain.as_mut_slice()),
            &Tag::from(tag),
        )
        .ok()
        .map(|()| plain)
}

/// The associated data every record is bound to.
fn header_bytes(blob: &[u8]) -> [u8; HEADER_LEN] {
    let mut aad = [0u8; HEADER_LEN];
    aad.copy_from_slice(&blob[..HEADER_LEN]);
    // The version byte says which layout the blob is in and is checked
    // against its length, not authenticated: an upgraded blob keeps the
    // duress record it cannot re-seal.
    aad[0] = AAD_VERSION;
    aad
}

/// Fills `out` with bytes drawn from `seed` for `label`: HMAC-SHA512
/// under the seed, one 64-byte block per counter. The seed is the
/// session key's own derivation, so these are as good as the session's
/// entropy and never repeat across writes.
fn draw(seed: &[u8; 32], label: u8, out: &mut [u8]) {
    for (block, chunk) in out.chunks_mut(64).enumerate() {
        let mut hmac = Hmac::<Sha512>::new_from_slice(seed).expect("HMAC accepts any key length");
        hmac.update(&[label, block as u8]);
        let mut wide = Secret::new(<[u8; 64]>::from(hmac.finalize().into_bytes()));
        chunk.copy_from_slice(&wide.expose()[..chunk.len()]);
        wide.zeroize();
    }
}

// ---------------------------------------------------------------------------
// The flow's state.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A blob-length buffer at the device's own cost, with `prefix` laid
    /// over the front of it the way an arbitrary file on the data
    /// partition would be.
    fn blob_with(prefix: &[u8]) -> Vec<u8> {
        let mut blob = vec![0u8; BLOB_LEN];
        blob[0] = VERSION;
        blob[1..5].copy_from_slice(&DEVICE_PARAMS.memory_kib.to_le_bytes());
        blob[5..9].copy_from_slice(&DEVICE_PARAMS.passes.to_le_bytes());
        blob[9..13].copy_from_slice(&DEVICE_PARAMS.lanes.to_le_bytes());
        blob[..prefix.len()].copy_from_slice(prefix);
        blob
    }

    #[test]
    fn a_blob_asking_for_more_work_than_the_device_does_is_not_a_blob() {
        // fuzz/artifacts/keep_open/argon2-cost-unclamped.bin: 16 GiB of
        // Argon2id memory, which is the allocation the first typed PIN
        // dies on.
        assert!(header(&blob_with(&[0x02, 0xff, 0xff, 0xff])).is_none());
        // argon2-cost-stall.bin: 448 MiB and 257 passes, which the first
        // typed PIN disappears into.
        assert!(
            header(&blob_with(&[
                0x02, 0x08, 0x00, 0x07, 0x00, 0x01, 0x01, 0x00
            ]))
            .is_none()
        );
    }

    #[test]
    fn a_blob_at_the_cost_this_build_writes_reads() {
        let header = header(&blob_with(&[])).expect("the device's own parameters");
        assert_eq!(header.cost, DEVICE_PARAMS);
    }
}
