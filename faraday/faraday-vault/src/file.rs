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

use alloc::vec::Vec;

use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

use crate::Cost;
use crate::kdbx::items::{self, Body, Place, Read, Unwritable};
use crate::kdbx::xml::{self, Document};
use crate::kdbx::{self, Binary, Cipher, Compression, Contents, Header, Kdf};
use crate::records::Record;

/// The plaintext sizes of a vault (§2): 64 KiB, 256 KiB, 1 MiB, 4 MiB.
pub const SIZES: [usize; 4] = [65_536, 262_144, 1_048_576, 4_194_304];

/// The size a new vault takes unless the person chooses (§2).
pub const DEFAULT_SIZE: usize = 262_144;

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

/// The length of a file Faraday seals at `size` (§2): 65,921, 262,529,
/// 1,048,961 and 4,194,797 bytes for the four sizes.
pub fn file_len(size: usize) -> usize {
    HEAD + size + framing(size)
}

/// What a file's clear header and length state, with no work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The version, cipher, compression, key derivation, cost and public
    /// custom data.
    pub header: Header,
    /// The file's length.
    pub len: usize,
    /// The size the length shows, worked out from the header's own length
    /// (public custom data moves it): `None` for a file no Faraday seal
    /// made, a KeePass app's save, whose plaintext is of any length, and
    /// any compressed file, whose plaintext can be longer than its length.
    pub size: Option<usize>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// The contents do not fit the size: the bytes over it, and the
    /// smallest larger size that holds them, when there is one.
    Full {
        /// How many bytes the plaintext is over the size.
        over: usize,
        /// The next size that holds it.
        next: Option<usize>,
    },
    /// A size that is not one of [`SIZES`].
    Size,
    /// The memory the work needs, in MiB, could not be allocated.
    Memory(u32),
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

/// Costs a SHA-256 of the file's header and nothing else. What a file's
/// header and length state (§4 steps 1 and 2): the header read, checked
/// against its hash, and its cost checked against §3's limits, and the
/// size the length shows.
pub fn inspect(file: &[u8]) -> Result<Facts, Error> {
    let header = kdbx::inspect(file)?;
    let head = kdbx::head_len(file)?;
    let size = if header.compression == Compression::None {
        SIZES
            .iter()
            .copied()
            .find(|&s| head + s + framing(s) == file.len())
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
    /// Remove a plan or signed amounts by its key.
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
    binaries: Vec<Binary>,
    passphrase: Zeroizing<Vec<u8>>,
}

/// Costs a hash of the header, an Argon2 run at the stated cost, then a
/// read of the document and of Faraday's items: opens `file` under `passphrase` (put in NFC,
/// which the opened vault keeps for its seals).
pub fn open(c: &Compute, file: &[u8], passphrase: &str) -> Result<Opened, Error> {
    let facts = inspect(file)?;
    let opened = kdbx::open(c, file, passphrase)?;
    let doc = Document::read(&opened.document, &opened.inner_key).map_err(Error::Xml)?;
    let read = items::read(c, &doc);
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
        binaries: opened.binaries,
        passphrase,
    })
}

/// Costs a document write for each item changed and a copy of the
/// document: `doc` with `changes` applied, in order, and the vault's own
/// values stamped for the next seal at `now`.
fn applied(
    c: &Compute,
    doc: &Document,
    read: &Read,
    changes: &[Change<'_>],
    now: Option<u64>,
) -> Result<Document, Error> {
    // The copy's buffers are wiping types, as the document's are.
    let mut copy = doc.clone();
    for change in changes {
        match change {
            Change::Put { place, body } => items::put(c, &mut copy, place, body)?,
            Change::Remove(uuid) => {
                items::remove(c, &mut copy, uuid)?;
            }
            Change::Plan { n, record } => items::put_plan(&mut copy, *n, record)?,
            Change::Signed(record) => items::put_signed(&mut copy, record)?,
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

    /// Why the vault is not changed or sealed, if it is not.
    pub fn read_only(&self) -> Option<ReadOnly> {
        self.read.why_read_only()
    }

    /// Costs what [`Opened::used`] costs: the size the next seal writes
    /// with `changes` applied, at `now`: the file's own, or for a file a
    /// KeePass app saved the smallest that holds its plaintext after the
    /// changes. It is the size [`seal`] writes, or [`Error::Full`] says it
    /// does not fit.
    pub fn next_size(
        &self,
        c: &Compute,
        changes: &[Change<'_>],
        now: Option<u64>,
    ) -> Result<usize, Error> {
        let doc = applied(c, &self.doc, &self.read, changes, now)?;
        let needs = kdbx::plain_needs(&self.binaries, doc.write(&[0u8; 64]).len());
        Ok(self.size_for(needs))
    }

    /// The size a plaintext of `needs` bytes is sealed at.
    fn size_for(&self, needs: usize) -> usize {
        self.facts.size.unwrap_or_else(|| smallest_holding(needs))
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

    /// Costs a document write and a read of each item put (linear, at most
    /// 4 MiB): the plaintext the next seal takes with `changes` applied,
    /// with the time of the seal counted. A seal with no clock takes 20
    /// bytes fewer.
    pub fn used(&self, c: &Compute, changes: &[Change<'_>]) -> Result<usize, Error> {
        let doc = applied(c, &self.doc, &self.read, changes, Some(0))?;
        Ok(kdbx::plain_needs(
            &self.binaries,
            doc.write(&[0u8; 64]).len(),
        ))
    }
}

/// The smallest size that holds `plain` bytes, or the largest.
fn smallest_holding(plain: usize) -> usize {
    SIZES
        .iter()
        .copied()
        .find(|&s| s >= plain)
        .unwrap_or(SIZES[SIZES.len() - 1])
}

/// The error for contents of `needs` bytes that do not fit `size`.
fn full(needs: usize, size: usize) -> Error {
    Error::Full {
        over: needs.saturating_sub(size),
        next: SIZES.iter().copied().find(|&s| s > size && s >= needs),
    }
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
/// and a document write for each item changed. A new file from `opened`
/// with `changes` applied to a copy of its document, `faraday.format`
/// stamped 1, the seal count one higher and the time `now` (Unix seconds;
/// none when the clock is not set). The file is sealed under `rekey`'s
/// passphrase when it has one, else under the one `opened` was opened with.
/// The document's protected values are encrypted under the inner key
/// `random` carries, the file is ChaCha20 and Argon2id with no
/// compression, and its plaintext is padded to [`Opened::next_size`]; a
/// KeePass app's save that grew past it takes the smallest larger size.
/// `opened` is not changed: a refusal leaves the vault open with its
/// changes. A read-only vault is refused, an empty passphrase and a step
/// outside [`STEPS_KIB`] too.
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
    let doc = applied(c, &opened.doc, &opened.read, changes, now)?;
    let xml = doc.write(kdbx::inner_key_of(random));
    let needs = kdbx::plain_needs(&opened.binaries, xml.len());
    let size = opened.size_for(needs);
    let contents = Contents {
        document: &xml,
        binaries: &opened.binaries,
        public_custom_data: opened.facts.header.public_custom_data.as_deref(),
    };
    let passphrase = match rekey.passphrase {
        Some(p) => p,
        None => core::str::from_utf8(&opened.passphrase).map_err(|_| kdbx::Error::Damaged)?,
    };
    kdbx::seal(
        c,
        passphrase,
        opened.next_cost_at(rekey.memory_kib),
        random,
        &contents,
        size,
    )
    .map_err(|e| match e {
        kdbx::Error::Full(n) => full(n, size),
        e => e.into(),
    })
}

/// Costs an Argon2id run at `cost`. A new vault: the root group with the
/// UUID `id`, named `label`, an empty database stamped as the first seal
/// (count 1, §8), sealed at `size` under `passphrase`.
#[expect(
    clippy::too_many_arguments,
    reason = "each is a value the new file states; the caller draws them all"
)]
pub fn create(
    c: &Compute,
    id: [u8; 16],
    label: &str,
    cost: Cost,
    size: usize,
    passphrase: &str,
    random: &[u8; kdbx::RANDOM_LEN],
    now: Option<u64>,
) -> Result<Vec<u8>, Error> {
    if !SIZES.contains(&size) {
        return Err(Error::Size);
    }
    let mut doc = Document::new(id, label);
    let time = now.and_then(items::utc_time);
    items::stamp(&mut doc, 1, time.as_deref())?;
    let xml = doc.write(kdbx::inner_key_of(random));
    let contents = Contents {
        document: &xml,
        binaries: &[],
        public_custom_data: None,
    };
    kdbx::seal(c, passphrase, cost, random, &contents, size).map_err(|e| match e {
        kdbx::Error::Full(n) => full(n, size),
        e => e.into(),
    })
}
