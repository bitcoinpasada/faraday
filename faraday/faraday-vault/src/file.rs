//! A file vault (`docs/VAULT.md` §1.1, §2 to §5, §7.1, §8): a KDBX 4 file
//! opened, changed and sealed the way the app does it, over the container
//! (`kdbx`), the XML document (`kdbx::xml`) and Faraday's items
//! (`kdbx::items`).
//!
//! - [`inspect`] reads a file's header and length, with no passphrase and
//!   no work: what the vault costs and which size it is (§4 steps 1 and 2).
//!   [`measure`] times that cost on this computer.
//! - [`open`] opens a file under its passphrase and reads what the app
//!   uses of it, giving an [`Opened`] that stays unchanged however the
//!   changes the app makes are applied.
//! - [`seal`] applies those changes to a copy of the document and writes a
//!   new file: every seal is a new file, from the one buffer of random
//!   bytes the caller passes (§5, F190 point 8).
//! - [`create`] writes a new vault (§8).
//!
//! The passphrase an [`Opened`] keeps is in its NFC form in a wiping
//! buffer; the copy of the document a seal changes, the document written
//! and the attachments are wiping buffers too.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use alloc::vec::Vec;

use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

use crate::Cost;
use crate::kdbx::items::{self, Body, Place, Read, Unwritable};
use crate::kdbx::xml::{self, Document};
use crate::kdbx::{self, Binary, Cipher, Components, Compression, Contents, Header, Kdf};
use crate::records::Record;

/// How many times the document the reader's uncharged text copies are
/// allowed for in an open's `room` ([`open_within`]): a few, since a pair
/// keeps its key and value twice and the text is read from the body.
const TEXT_COPIES: usize = 4;

/// The step a vault's plaintext grows in (§2, F206): 32 KiB. A new vault
/// is one step, and every seal pads to the smallest multiple of it that
/// holds the contents.
pub const STEP: usize = 32 * 1024;

/// The most a vault's plaintext may be (§2, F206): 128 MiB.
pub const CEILING: usize = kdbx::MAX_PLAIN;

/// The memory steps Faraday writes, in KiB (§3): 64 MiB to 1 GiB.
pub const STEPS_KIB: [u32; 5] = [FIRST_STEP_KIB, 131_072, 262_144, 524_288, LAST_STEP_KIB];

/// The smallest of [`STEPS_KIB`], in KiB.
pub const FIRST_STEP_KIB: u32 = 65_536;
/// The largest of [`STEPS_KIB`], in KiB.
pub const LAST_STEP_KIB: u32 = 1_048_576;

/// The passes Faraday writes (§3).
pub const PASSES: u32 = 3;
/// The lanes Faraday writes (§3).
pub const LANES: u32 = 4;

/// The length of the block a file's plaintext is framed in.
const BLOCK: usize = 1024 * 1024;
/// What each block of the stream adds: its HMAC and its length.
const FRAME: usize = 36;
/// The header, its hash and its HMAC when Faraday writes the file and it
/// carries no public custom data (§1.1).
const HEAD: usize = 313;

/// What the stream adds to a plaintext of `size` bytes: a frame for each
/// block and one for the empty block after them.
fn framing(size: usize) -> usize {
    FRAME * (size.div_ceil(BLOCK) + 1)
}

/// The length of a file Faraday seals at `size` (§2): 33,153 bytes for a
/// new vault and 134,222,685 for the largest.
pub fn file_len(size: usize) -> usize {
    HEAD + size + framing(size)
}

/// What a file's clear header and length state, with no work.
#[derive(Debug, PartialEq, Eq)]
pub struct Facts {
    /// The version, cipher, compression, key derivation, cost and public
    /// custom data.
    pub header: Header,
    /// The file's length.
    pub len: usize,
    /// The plaintext the length shows, worked out from the header's own
    /// length (public custom data moves it): `None` for a file no Faraday
    /// seal made, a KeePass app's save, whose plaintext is not a multiple
    /// of [`STEP`], and any compressed file, whose plaintext can be longer
    /// than its length.
    pub size: Option<usize>,
}

#[expect(
    clippy::disallowed_methods,
    reason = "a public value: its fields hold no secret"
)]
impl Clone for Facts {
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            len: self.len,
            size: self.size,
        }
    }
}

impl Facts {
    /// Whether the file is saved in other settings than Faraday writes:
    /// not 4.0, ChaCha20, Argon2id and no compression. The head says so
    /// (§7.1).
    pub fn other_settings(&self) -> bool {
        let h = &self.header;
        h.minor != 0
            || h.cipher != Cipher::ChaCha20
            || h.kdf != Kdf::Argon2id
            || h.compression != Compression::None
    }
}

/// Why a file could not be opened, sealed or made.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// The container: not KDBX, a version or setting not taken, a wrong
    /// passphrase, damage (§7.1).
    Kdbx(kdbx::Error),
    /// The XML document: what it holds cannot be carried or placed (§7.1).
    Xml(xml::Refused),
    /// A change that could not be applied.
    Unwritable(Unwritable),
    /// The vault is read-only ([`Opened::read_only`]) and is not sealed.
    ReadOnly,
    /// The contents are past [`CEILING`]: the bytes over it.
    Full {
        /// How many bytes the plaintext is over the ceiling.
        over: usize,
    },
    /// The memory the work needs, in MiB, could not be allocated.
    Memory(u32),
}

#[expect(
    clippy::disallowed_methods,
    reason = "a public value: its fields hold no secret"
)]
impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Self::Kdbx(a0) => Self::Kdbx(*a0),
            Self::Xml(a0) => Self::Xml(a0.clone()),
            Self::Unwritable(a0) => Self::Unwritable(a0.clone()),
            Self::ReadOnly => Self::ReadOnly,
            Self::Full { over } => Self::Full { over: *over },
            Self::Memory(a0) => Self::Memory(*a0),
        }
    }
}

impl From<kdbx::Error> for Error {
    fn from(e: kdbx::Error) -> Self {
        match e {
            kdbx::Error::Memory(mib) => Self::Memory(mib),
            e => Self::Kdbx(e),
        }
    }
}

impl From<Unwritable> for Error {
    fn from(e: Unwritable) -> Self {
        match e {
            Unwritable::ReadOnly => Self::ReadOnly,
            e => Self::Unwritable(e),
        }
    }
}

/// The plaintext whose stream is `stream` bytes (frames included), if it
/// is a multiple of [`STEP`] up to the ceiling.
fn stepped_plain(stream: usize) -> Option<usize> {
    // The number of 1 MiB blocks decides the frames: try each.
    (0..=CEILING / BLOCK + 1).find_map(|blocks| {
        let plain = stream.checked_sub(FRAME * (blocks + 1))?;
        (plain > 0
            && plain <= CEILING
            && plain.div_ceil(BLOCK) == blocks
            && plain.is_multiple_of(STEP))
        .then_some(plain)
    })
}

/// Costs a SHA-256 of the file's header and nothing else. What a file's
/// header and length state (§4 steps 1 and 2): the header read, checked
/// against its hash, and its cost checked against §3's limits, and the
/// size the length shows.
pub fn inspect(file: &[u8]) -> Result<Facts, Error> {
    let header = kdbx::inspect(file)?;
    let head = kdbx::head_len(file)?;
    let size = if header.compression == Compression::None {
        stepped_plain(file.len().saturating_sub(head))
    } else {
        None
    };
    Ok(Facts {
        header,
        len: file.len(),
        size,
    })
}

/// Costs an Argon2 run at the stated cost, thrown away: how long an
/// unlock of this file takes on this computer, to show before the
/// passphrase is asked for (§3). The machine lacking the memory is
/// [`Error::Memory`]: the unlock is not offered (§4 step 2).
pub fn measure(c: &Compute, facts: &Facts) -> Result<(), Error> {
    kdbx::measure(c, &facts.header).map_err(Error::from)
}

pub use crate::kdbx::items::ReadOnly;

/// A change the app makes, applied to a copy of the document at a seal.
pub enum Change<'a> {
    /// Add an item, or change the one with this UUID.
    Put {
        /// Where it goes and what it is called.
        place: Place<'a>,
        /// What it holds.
        body: &'a Body,
    },
    /// Remove the entry with this UUID.
    Remove([u8; 16]),
    /// Write a backup plan as `faraday.plan.{n}`.
    Plan {
        /// Its number, from 1 ([`items::next_plan`] gives a new one).
        n: u32,
        /// The plan record.
        record: &'a Record,
    },
    /// Write a transaction's signed amounts.
    Signed(&'a Record),
    /// Write a transaction's nonce record.
    Nonces(&'a Record),
    /// Remove a plan, signed amounts or a nonce record by its key.
    RemoveCustom(&'a str),
    /// Rename the vault.
    Label(&'a str),
}

/// A file opened: what it states, what the app reads of it, and what a
/// seal needs of it. Changing it is [`seal`]'s work, on a copy; this is
/// not changed.
pub struct Opened {
    facts: Facts,
    doc: Document,
    read: Read,
    /// The entries' UUIDs in `doc` as read.
    index: items::Index,
    binaries: Vec<Binary>,
    /// The XML document's length as the file held it, measured once when it
    /// opened.
    document_len: usize,
    passphrase: Zeroizing<Vec<u8>>,
    /// The key file's key and the YubiKey's secret the file was opened
    /// with, which a seal puts back in the composite key (§3.2).
    components: Components,
}

/// How many changes [`Opened::check`] has looked at in this process.
static CHECKED: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// How many changes [`Opened::check`] has looked at in this process, by
/// anything: a test that an edit checks the one change it makes compares two
/// readings. As with `kdbx::xml::writes`, exact where a test runs alone in
/// its process.
pub fn checked() -> usize {
    CHECKED.load(core::sync::atomic::Ordering::Relaxed)
}

/// Costs a hash of the header, an Argon2 run at the stated cost, then a
/// read of the document and of Faraday's items: opens `file` under `passphrase` (put in NFC,
/// which the opened vault keeps for its seals).
pub fn open(c: &Compute, file: &[u8], passphrase: &str) -> Result<Opened, Error> {
    open_within(
        c,
        file,
        passphrase,
        &mut |_| Ok(()),
        &mut |_, _| Ok(None),
        None,
    )
}

/// Costs what [`open`] costs. [`open`] for a caller with a memory budget:
/// `budget` is asked, for a compressed file, with the plaintext length it
/// states once the HMACs have verified, before the buffer of that length is
/// allocated; `Err(mib)` is the memory that was not found (F206 point 5).
/// `parsed` is asked once the inner header is read, with the XML document's
/// length and the attachments' together, before the document is copied out
/// of the body or parsed (F206 point 4); `Ok(Some(bytes))` caps `room` at
/// that, so the open takes no more than the figure the check counted.
/// `room` is the bytes this open may take beyond what the caller already
/// holds (the app, a copy of the file): what the open holds besides the
/// document's structures comes out of it (the file, the body inflated, the
/// document's own buffer and the text the reader copies), and the rest is
/// the bytes the structures may take, as [`Document::read_within`] has it:
/// past it the open ends with [`Error::Memory`] and no more is taken; the
/// items read from it, the index and the records the app builds then count
/// against it too.
pub fn open_within(
    c: &Compute,
    file: &[u8],
    passphrase: &str,
    budget: &mut dyn FnMut(usize) -> Result<(), u32>,
    parsed: &mut dyn FnMut(usize, usize) -> Result<Option<usize>, u32>,
    room: Option<usize>,
) -> Result<Opened, Error> {
    open_with(
        c,
        file,
        passphrase,
        Components::none(),
        budget,
        parsed,
        room,
    )
}

/// Costs what [`open`] costs. [`open_within`] for a file that also needs a
/// key file or a YubiKey's secret (§3.2, F208): `components`, which the
/// opened vault keeps for its seals (moved in: no second copy is made).
pub fn open_with(
    c: &Compute,
    file: &[u8],
    passphrase: &str,
    components: Components,
    budget: &mut dyn FnMut(usize) -> Result<(), u32>,
    parsed: &mut dyn FnMut(usize, usize) -> Result<Option<usize>, u32>,
    room: Option<usize>,
) -> Result<Opened, Error> {
    let facts = inspect(file)?;
    // `parsed` may also give the most the open may take, which holds
    // whatever is free, so a crafted file takes no more than it counted.
    let cap = core::cell::Cell::new(None);
    let opened = kdbx::open_with(
        c,
        file,
        passphrase,
        &components,
        budget,
        &mut |document, attachments| {
            cap.set(parsed(document, attachments)?);
            Ok(())
        },
    )?;
    let room = match (room, cap.get()) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let attachments = opened
        .binaries
        .iter()
        .fold(0usize, |n, b| n.saturating_add(b.data.len()));
    let document = opened.document.len();
    // What the open holds beside the structures: the file, the body (the
    // document and the attachments) and the document's own buffer, and the
    // text the reader copies.
    let model = room.map(|room| {
        room.saturating_sub(file.len())
            .saturating_sub(attachments)
            .saturating_sub(document.saturating_mul(2 + TEXT_COPIES))
    });
    let doc =
        Document::read_within(&opened.document, &opened.inner_key, model).map_err(|e| match e {
            xml::Refused::Memory(bytes) => {
                Error::Memory(u32::try_from(bytes.div_ceil(1 << 20)).unwrap_or(u32::MAX))
            }
            e => Error::Xml(e),
        })?;
    let read = items::read(c, &doc);
    let index = items::Index::of(&doc);
    // What the later steps hold, against the budget the reader ran under:
    // the items read, the index, and the records the app builds from the
    // items (as many bytes again).
    if let Some(model) = model {
        let held = doc
            .charged()
            .saturating_add(read.held().saturating_mul(2))
            .saturating_add(index.held());
        if held > model {
            return Err(Error::Memory(
                u32::try_from(held.div_ceil(1 << 20)).unwrap_or(u32::MAX),
            ));
        }
    }
    let nfc = crate::nfc(passphrase).map_err(Error::Memory)?;
    let mut passphrase = Zeroizing::new(Vec::new());
    passphrase
        .try_reserve_exact(nfc.len())
        .map_err(|_| Error::Memory(1))?;
    passphrase.extend_from_slice(nfc.as_bytes());
    Ok(Opened {
        facts,
        doc,
        read,
        index,
        binaries: opened.binaries,
        document_len: opened.document.len(),
        passphrase,
        components,
    })
}

/// Costs a document write for each item changed and a copy of the
/// document: `doc` with `changes` applied, in order, and the vault's own
/// values stamped for the next seal at `now`.
#[expect(
    clippy::disallowed_methods,
    reason = "the document is copied into wiping buffers, as the one it came from"
)]
fn applied(
    c: &Compute,
    doc: &Document,
    read: &Read,
    changes: &[Change<'_>],
    now: Option<u64>,
) -> Result<Document, Error> {
    // The copy's buffers are wiping types, as the document's are.
    let mut copy = doc.clone();
    // An entry no earlier change of the batch touched reads as it did
    // before it: its readability is `read`'s, not worked out again from
    // every entry of the copy.
    let mut touched: Vec<[u8; 16]> = Vec::new();
    let mut known = |uuid: &[u8; 16]| {
        let first = !touched.contains(uuid);
        touched.push(*uuid);
        first.then_some(read)
    };
    for change in changes {
        match change {
            Change::Put { place, body } => {
                items::put_known(c, &mut copy, place, body, known(&place.uuid))?
            }
            Change::Remove(uuid) => {
                items::remove_known(c, &mut copy, uuid, known(uuid))?;
            }
            Change::Plan { n, record } => items::put_plan(&mut copy, *n, record)?,
            Change::Signed(record) => items::put_signed(&mut copy, record)?,
            Change::Nonces(record) => items::put_nonces(&mut copy, record)?,
            Change::RemoveCustom(key) => {
                items::remove_custom(&mut copy, key)?;
            }
            Change::Label(label) => {
                if read.read_only() {
                    return Err(Error::ReadOnly);
                }
                copy.set_label(label)
                    .map_err(|_| Error::Unwritable(Unwritable::Unreadable))?;
            }
        }
    }
    let time = now.and_then(items::utc_time);
    items::stamp(&mut copy, read.next_seal(), time.as_deref())?;
    Ok(copy)
}

impl Opened {
    /// A lookup. The bytes a seal of this vault has to hold beside the
    /// opened one, as the file held them when it opened: the XML document's
    /// length and the attachments' together (F206 point 4).
    pub fn held(&self) -> (usize, usize) {
        let attachments = self
            .binaries
            .iter()
            .fold(0usize, |n, b| n.saturating_add(b.data.len()));
        (self.document_len, attachments)
    }

    /// What the app reads of the vault: its items, plans, signed amounts,
    /// what is unreadable, the seal count and the rest.
    pub fn read(&self) -> &Read {
        &self.read
    }

    /// The document as read, without a change.
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// What the file's header and length stated.
    pub fn facts(&self) -> &Facts {
        &self.facts
    }

    /// The vault's identifier: the root group's UUID as 32 lower-case hex
    /// digits.
    pub fn vault_id(&self) -> [u8; 32] {
        self.doc.vault_id()
    }

    /// A lookup. Whether the file was opened with a key file and with a
    /// YubiKey's secret, which its seals write again.
    pub fn components_held(&self) -> (bool, bool) {
        (
            self.components.has_key_file(),
            self.components.has_yubikey(),
        )
    }

    /// Why the vault is not changed or sealed, if it is not.
    pub fn read_only(&self) -> Option<ReadOnly> {
        self.read.why_read_only()
    }

    /// Costs what [`Opened::used`] costs: the size the next seal writes
    /// with `changes` applied, at `now`: the smallest multiple of [`STEP`]
    /// that holds the plaintext. It is the size [`seal`] writes, or
    /// [`Error::Full`] says the contents are past [`CEILING`].
    pub fn next_size(
        &self,
        c: &Compute,
        changes: &[Change<'_>],
        now: Option<u64>,
    ) -> Result<usize, Error> {
        let doc = applied(c, &self.doc, &self.read, changes, now)?;
        let xml = doc.write(&[0u8; 64]);
        size_for(kdbx::plain_needs(&self.sealed_binaries(&xml), xml.len()))
    }

    /// The cost the next seal writes: the memory step at or above the
    /// file's own, at most 1 GiB, and 3 passes, or the file's when it has
    /// more (VAULT §7.1), over 4 lanes.
    pub fn next_cost(&self) -> Cost {
        self.next_cost_at(None)
    }

    /// The cost a seal writes with `step` (one of [`STEPS_KIB`], in KiB) as
    /// its memory, or [`Opened::next_cost`]'s own when there is none: the
    /// passes stay the file's or [`PASSES`], over 4 lanes (§3.1).
    pub fn next_cost_at(&self, step: Option<u32>) -> Cost {
        let have = &self.facts.header.cost;
        let memory_kib = step.unwrap_or_else(|| {
            STEPS_KIB
                .iter()
                .copied()
                .find(|&step| step >= have.memory_kib)
                .unwrap_or(STEPS_KIB[STEPS_KIB.len() - 1])
        });
        Cost {
            memory_kib,
            passes: have.passes.max(PASSES),
            lanes: LANES,
        }
    }

    /// Whether an entry of the file has this UUID: a lookup. What a
    /// stand-in UUID for a new entry must not equal.
    pub fn holds_entry(&self, uuid: &[u8; 16]) -> bool {
        self.index.holds(uuid)
    }

    /// Costs a parse of a wallet's text for each item put. Whether
    /// `changes` would be refused as unwritable, found against the
    /// document as read and its index of UUIDs, each from the one entry it
    /// touches: no copy, no write and no walk of the document. A refusal
    /// that only the changes applied in order show comes at [`seal`].
    pub fn check(&self, c: &Compute, changes: &[Change<'_>]) -> Result<(), Error> {
        for change in changes {
            if let Change::Label(_) = change
                && self.read_only().is_some()
            {
                return Err(Error::ReadOnly);
            }
            CHECKED.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            items::check(c, &self.doc, &self.read, &self.index, change)?;
        }
        Ok(())
    }

    /// Costs a copy of the document, a read of each item put and a count of
    /// the document (linear, at most the vault's largest, [`CEILING`]): the plaintext the next seal takes with `changes` applied,
    /// with the time of the seal counted. A seal with no clock takes 20
    /// bytes fewer.
    pub fn used(&self, c: &Compute, changes: &[Change<'_>]) -> Result<usize, Error> {
        let doc = applied(c, &self.doc, &self.read, changes, Some(0))?;
        let xml = doc.write(&[0u8; 64]);
        Ok(kdbx::plain_needs(&self.sealed_binaries(&xml), xml.len()))
    }

    /// Costs a count of the document's bytes, linear in its size and with
    /// no copy of it: what [`Opened::used`] gives for no change, or more
    /// by the attachments no entry refers to, the vault's own values stamped
    /// in place and put back as they were.
    #[expect(
        clippy::disallowed_methods,
        reason = "the vault's custom data is copied into wiping buffers"
    )]
    pub fn base_used(&mut self) -> Result<usize, Error> {
        let saved = self.doc.custom().clone();
        let time = items::utc_time(0);
        let stamped = items::stamp(&mut self.doc, self.read.next_seal(), time.as_deref());
        let len = self.doc.written_len();
        *self.doc.custom_mut() = saved;
        stamped?;
        Ok(kdbx::plain_needs(&kdbx::Binaries::all(&self.binaries), len))
    }

    /// Costs a read of `xml` (nothing without attachments): the attachments
    /// a seal of the document `xml` writes (VAULT §7.1).
    fn sealed_binaries(&self, xml: &[u8]) -> kdbx::Binaries<'_> {
        kdbx::kept_binaries(&self.binaries, xml)
    }
}

/// Costs a document write and a read of each item put. The plaintext the
/// first seal after [`create`] would take of a new vault named `label`
/// once the `Put` changes in `changes` are applied, with the time of the
/// seal counted: what says whether items fit a size before the vault is
/// made. Any other change is refused.
pub fn used_new(c: &Compute, label: &str, changes: &[Change<'_>]) -> Result<usize, Error> {
    let mut doc = Document::new([0u8; 16], label);
    for change in changes {
        match change {
            Change::Put { place, body } => items::put(c, &mut doc, place, body)?,
            _ => return Err(Error::Unwritable(Unwritable::Unreadable)),
        }
    }
    let time = items::utc_time(0);
    items::stamp(&mut doc, 2, time.as_deref())?;
    Ok(kdbx::plain_needs(
        &kdbx::Binaries::all(&[]),
        doc.write(&[0u8; 64]).len(),
    ))
}

/// The size a plaintext of `needs` bytes is sealed at: the smallest
/// multiple of [`STEP`] that holds it, or [`Error::Full`] past [`CEILING`].
fn size_for(needs: usize) -> Result<usize, Error> {
    if needs > CEILING {
        return Err(Error::Full {
            over: needs - CEILING,
        });
    }
    Ok(needs.div_ceil(STEP).max(1) * STEP)
}

/// What a seal changes about the key besides the contents (§3, §3.1): a
/// new passphrase, a new memory step, both or neither.
#[derive(Clone, Copy, Default)]
pub struct Rekey<'a> {
    /// The passphrase the file is sealed under, in place of the one it was
    /// opened with; put in NFC, never empty.
    pub passphrase: Option<&'a str>,
    /// The memory the Argon2id run takes, in KiB: one of [`STEPS_KIB`].
    pub memory_kib: Option<u32>,
    /// The key file's key the seal writes: `Some(None)` takes the key file
    /// out, `Some(Some(key))` adds or replaces it; none keeps the file's own.
    pub key_file: Option<Option<&'a [u8; 32]>>,
    /// Takes the YubiKey's secret out of the seal.
    pub drop_yubikey: bool,
}

/// Costs what [`seal_with`] costs. [`seal_with`] with no new passphrase and
/// no new step.
pub fn seal(
    c: &Compute,
    opened: &Opened,
    changes: &[Change<'_>],
    random: &[u8; kdbx::RANDOM_LEN],
    now: Option<u64>,
) -> Result<Vec<u8>, Error> {
    seal_with(c, opened, changes, Rekey::default(), random, now)
}

/// Costs an Argon2id run at [`Opened::next_cost_at`] the step of `rekey`,
/// a document write for each item changed, and a read of the document
/// and of the file made, written and read again. A new file from `opened`
/// with `changes` applied to a copy of its document, `faraday.format`
/// stamped 1, the seal count one higher and the time `now` (Unix seconds;
/// none when the clock is not set). The file is sealed under `rekey`'s
/// passphrase when it has one, else under the one `opened` was opened with.
/// The document's protected values are encrypted under the inner key
/// `random` carries, the file is ChaCha20 and Argon2id with no
/// compression, and its plaintext is padded to [`Opened::next_size`], which
/// is smaller than the file's own when the contents are.
/// `opened` is not changed: a refusal leaves the vault open with its
/// changes. A read-only vault is refused, an empty passphrase and a step
/// outside [`STEPS_KIB`] too. The document written is read and written
/// again, and the file made is opened again with the keys it was made
/// under (no second Argon2id run): a file that would not read back is
/// refused as [`kdbx::Error::ReadBack`].
pub fn seal_with(
    c: &Compute,
    opened: &Opened,
    changes: &[Change<'_>],
    rekey: Rekey<'_>,
    random: &[u8; kdbx::RANDOM_LEN],
    now: Option<u64>,
) -> Result<Vec<u8>, Error> {
    if opened.read_only().is_some() {
        return Err(Error::ReadOnly);
    }
    if rekey.memory_kib.is_some_and(|m| !STEPS_KIB.contains(&m)) {
        return Err(kdbx::Error::Cost.into());
    }
    // Argon2id runs, and its memory is freed, before the seal builds
    // anything (F206 point 6).
    let passphrase = match rekey.passphrase {
        Some(p) => p,
        None => core::str::from_utf8(&opened.passphrase).map_err(|_| kdbx::Error::Damaged)?,
    };
    let components = opened
        .components
        .changed(rekey.key_file, rekey.drop_yubikey);
    let sealing = kdbx::derive_with(
        c,
        passphrase,
        &components,
        opened.next_cost_at(rekey.memory_kib),
        random,
    )?;
    let doc = applied(c, &opened.doc, &opened.read, changes, now)?;
    let xml = doc.write(kdbx::inner_key_of(random));
    // The copy with the changes is not held while the text is read back
    // and the file is made (F206 point 6).
    drop(doc);
    read_back(&xml, kdbx::inner_key_of(random))?;
    let binaries = opened.sealed_binaries(&xml);
    let size = size_for(kdbx::plain_needs(&binaries, xml.len()))?;
    let contents = Contents {
        document: &xml,
        binaries,
        public_custom_data: opened.facts.header.public_custom_data.as_deref(),
    };
    Ok(kdbx::seal_keyed(sealing, random, &contents, size)?)
}

/// Costs a parse of the document and a write of it again, linear in its
/// size (at most the vault's largest, [`CEILING`]). Reads the document `xml` a
/// seal has written, under the inner key `inner_key`, and writes what it
/// read: [`kdbx::Error::ReadBack`] unless it reads and the bytes written
/// again are the bytes given, so a document that does not parse, or that
/// the reader takes otherwise than it was written, is refused before the
/// file is made. A value the writer got wrong in a way that reads back the
/// same is not seen here.
pub(crate) fn read_back(xml: &[u8], inner_key: &[u8]) -> Result<(), Error> {
    let doc = Document::read(xml, inner_key).map_err(|_| kdbx::Error::ReadBack)?;
    let again = doc.write(inner_key);
    if again.as_slice() != xml {
        return Err(kdbx::Error::ReadBack.into());
    }
    Ok(())
}

/// Costs an Argon2id run at `cost`. A new vault: the root group with the
/// UUID `id`, named `label`, an empty database stamped as the first seal
/// (count 1, §8), one [`STEP`] of plaintext, under `passphrase`.
pub fn create(
    c: &Compute,
    id: [u8; 16],
    label: &str,
    cost: Cost,
    passphrase: &str,
    random: &[u8; kdbx::RANDOM_LEN],
    now: Option<u64>,
) -> Result<Vec<u8>, Error> {
    create_holding(
        c,
        id,
        label,
        cost,
        passphrase,
        &Components::none(),
        random,
        now,
        &[],
    )
}

/// Costs an Argon2id run at `cost` and a document write for each item put.
/// A new vault as [`create`] makes it (first seal, count 1), with `changes`
/// put in it, each a [`Change::Put`], sealed at the smallest multiple of [`STEP`]
/// that holds it. The file needs `components` after the passphrase (§3.2).
#[expect(
    clippy::too_many_arguments,
    reason = "each is a value the new file states; the caller draws them all"
)]
pub fn create_holding(
    c: &Compute,
    id: [u8; 16],
    label: &str,
    cost: Cost,
    passphrase: &str,
    components: &Components,
    random: &[u8; kdbx::RANDOM_LEN],
    now: Option<u64>,
    changes: &[Change<'_>],
) -> Result<Vec<u8>, Error> {
    let mut doc = Document::new(id, label);
    for change in changes {
        match change {
            Change::Put { place, body } => items::put(c, &mut doc, place, body)?,
            _ => return Err(Error::Unwritable(Unwritable::Unreadable)),
        }
    }
    let time = now.and_then(items::utc_time);
    items::stamp(&mut doc, 1, time.as_deref())?;
    let xml = doc.write(kdbx::inner_key_of(random));
    let size = size_for(kdbx::plain_needs(&kdbx::Binaries::all(&[]), xml.len()))?;
    let contents = Contents {
        document: &xml,
        binaries: kdbx::Binaries::all(&[]),
        public_custom_data: None,
    };
    Ok(kdbx::seal_with(
        c, passphrase, components, cost, random, &contents, size,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contents_past_what_a_vault_holds_are_refused_by_how_many_bytes() {
        assert_eq!(size_for(CEILING + 1234), Err(Error::Full { over: 1234 }));
    }
}
