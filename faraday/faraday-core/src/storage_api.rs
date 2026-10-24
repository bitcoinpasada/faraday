//! The shell's storage protocol: the sticks it sees, what it tells the app
//! about them, and what the app asks it to do.

use crate::Unrenamed;

/// A stick or card the shell sees, with the files on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickInfo {
    /// Where the shell finds it; the app hands this back unchanged.
    pub id: String,
    /// The name a person knows it by: its volume label.
    pub label: String,
    /// Whether it is the medium the machine booted from.
    pub boot: bool,
    /// Its FAT volume serial number, when the shell can read one: what
    /// tells two sticks of the same label apart (`docs/VAULT.md` §6).
    pub serial: Option<u32>,
    /// The files at its top level, with their sizes.
    pub files: Vec<(String, u64)>,
}

/// A boot partition the boot copier has, while the app upgrades a stick
/// (`docs/history/PLAN.md` §5.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootPart {
    /// The copier's name for it; the app hands this back unchanged.
    pub id: String,
    /// Its size in bytes.
    pub size: u64,
    /// The Faraday release string found on it, or `None` for a stick
    /// made before Faraday put one in its kernel.
    pub release: Option<String>,
    /// The source was read from it.
    pub source: bool,
}

/// What the shell tells the app about storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageEvent {
    /// Every stick present now. Sent at start and whenever the set or a
    /// stick's files change.
    Sticks(Vec<StickInfo>),
    /// A file the app asked for.
    Read {
        /// The stick.
        stick: String,
        /// The file.
        name: String,
        /// Its contents.
        bytes: Vec<u8>,
    },
    /// A read that failed.
    ReadFailed {
        /// The stick.
        stick: String,
        /// The file.
        name: String,
        /// Why.
        reason: String,
    },
    /// A write that was read back and matched.
    Written {
        /// The stick.
        stick: String,
        /// The Outbox file's name.
        name: String,
        /// The name it was written under on the stick.
        wrote_as: String,
    },
    /// A write that failed.
    WriteFailed {
        /// The stick.
        stick: String,
        /// The Outbox file's name.
        name: String,
        /// Why.
        reason: String,
    },
    /// What the app asked to be kept across a lock could not be kept. A
    /// lock with a key that waits for its backup does not go ahead.
    NotKept,
    /// A file deleted from a stick, zeros over it first.
    Wiped {
        /// The stick.
        stick: String,
        /// The file.
        name: String,
    },
    /// A delete that failed; the file may hold zeros in part.
    WipeFailed {
        /// The stick.
        stick: String,
        /// The file.
        name: String,
        /// Why.
        reason: String,
    },
    /// A rename's answer ([`StorageCommand::Rename`]): done, or why not.
    Renamed {
        /// The stick.
        stick: String,
        /// The file's name before.
        from: String,
        /// The name asked for.
        to: String,
        /// Why it was refused; `None` once renamed.
        refused: Option<Unrenamed>,
    },
    /// A file's SHA-256 ([`StorageCommand::Hash`]), or why it was not read.
    Hashed {
        /// The stick.
        stick: String,
        /// The file.
        name: String,
        /// Its SHA-256.
        sha: Result<[u8; 32], String>,
    },
    /// A stick's free space and whole data space, in bytes; both 0 for a
    /// folder standing for a stick.
    StickSpace {
        /// The stick.
        stick: String,
        /// Bytes no file holds.
        free: u64,
        /// Bytes in all.
        size: u64,
    },
    /// How far zeroing a stick has gone: the cluster the next step starts
    /// at, of `total`, and whether every free cluster is zeros.
    Zeroing {
        /// The stick.
        stick: String,
        /// The next step's first cluster.
        next: u32,
        /// The volume's clusters.
        total: u32,
        /// Done.
        done: bool,
    },
    /// Zeroing that stopped with an error, or a space it could not read.
    ZeroFailed {
        /// The stick.
        stick: String,
        /// Why.
        reason: String,
    },
    /// A picture on a stick: the QR codes the disk process read from it,
    /// and its bytes as they are. The app keeps and writes the bytes and
    /// never decodes them (`docs/history/PLAN.md` §12 item 6).
    QrRead {
        /// The image's name.
        name: String,
        /// The picture's bytes, unchanged.
        bytes: Vec<u8>,
        /// Each code's payload.
        payloads: Vec<Vec<u8>>,
    },
    /// The time on this computer's clock, Unix seconds: what a GPG key or
    /// signature states as its creation time.
    Clock {
        /// Seconds since 1970.
        unix_secs: u64,
    },
    /// The memory this machine has free, as the shell measures it.
    Memory {
        /// MiB.
        available_mib: u32,
    },
    /// A PDF the app asked for was saved.
    Printed {
        /// Where.
        path: String,
    },
    /// A PDF could not be saved, or this shell does not print.
    PrintFailed {
        /// Why.
        reason: String,
    },
    /// An input device the shell holds back until a person says it is
    /// theirs (`docs/history/PLAN.md` §4.6).
    NewInput {
        /// The shell's number for it.
        id: u32,
        /// What it calls itself.
        name: String,
        /// It types.
        keyboard: bool,
        /// It points.
        pointer: bool,
    },
    /// A character a held-back keyboard typed.
    InputTyped {
        /// The device.
        id: u32,
        /// The character.
        ch: char,
    },
    /// The cameras the shell can open: an id it takes back from
    /// [`Faraday::take_camera`] and the name the camera gives.
    Cameras(Vec<(String, String)>),
    /// What one scan pass saw of a code: where it was in the frame, how
    /// fine its modules are, and whether it read (`docs/QR.md` §1).
    QrSeen {
        /// The frame's width.
        width: u16,
        /// The frame's height.
        height: u16,
        /// The corners: top-left, top-right, bottom-right, bottom-left.
        corners: [(u16, u16); 4],
        /// One module's side, in tenths of a frame pixel.
        module_tenths: u16,
        /// It read.
        read: bool,
    },
    /// A held-back or ignored device was unplugged.
    InputGone {
        /// The device.
        id: u32,
    },
    /// The online app's Downloads folder, for Transfer: where it is, its
    /// files newest first with their sizes, and whether the shell can
    /// open it in a file manager. Sent at start and when it changes.
    HostFiles {
        /// The folder's path.
        dir: String,
        /// Its files, newest first.
        files: Vec<(String, u64)>,
        /// The shell opens it on [`StorageCommand::OpenDownloads`].
        opens: bool,
    },
    /// A file dragged over the online app's window (`true`), or the
    /// drag gone without a drop (`false`).
    Hovered(bool),
    /// A file dropped on the online app's window.
    Dropped {
        /// Its path.
        path: String,
        /// Its size in bytes.
        size: u64,
    },
    /// A file of this computer's the app asked for
    /// ([`StorageCommand::ReadHost`]).
    HostRead {
        /// Its path.
        path: String,
        /// Its contents.
        bytes: Vec<u8>,
    },
    /// A read of this computer's files that failed.
    HostReadFailed {
        /// The path.
        path: String,
        /// Why.
        reason: String,
    },
    /// A file received by Transfer was saved
    /// ([`StorageCommand::SaveDownload`]).
    Saved {
        /// Where.
        path: String,
    },
    /// A received file could not be saved, or this shell does not save.
    SaveFailed {
        /// Why.
        reason: String,
    },
    /// The boot partitions the boot copier has now, sent while
    /// [`Faraday::upgrading`] and whenever they change.
    Boots(Vec<BootPart>),
    /// The source was read: the boot partition holding the running
    /// Faraday.
    BootSource {
        /// The partition.
        id: String,
        /// The running kernel's release string, which it holds.
        release: String,
        /// Its size in bytes.
        size: u64,
    },
    /// No source was taken.
    BootSourceFailed {
        /// Why.
        reason: String,
    },
    /// A boot partition was written, read back and matched.
    BootWritten {
        /// The partition.
        id: String,
        /// The release string it now carries.
        release: String,
    },
    /// A boot partition was not written.
    BootWriteFailed {
        /// The partition.
        id: String,
        /// Why.
        reason: String,
        /// The stick went during the write: it does not boot until it is
        /// written again.
        pulled: bool,
    },
    /// A stick the shell cannot use as the backup flow needs: not a FAT32
    /// stick as Faraday reads one, or one that cannot be written
    /// (`docs/BACKUP-FLOW.md` §4.2).
    StickProblem {
        /// The stick.
        stick: String,
        /// It is read-only (else it is not FAT32).
        read_only: bool,
    },
    /// The Inbox and Outbox the previous process left, after a lock.
    Restored {
        /// Inbox files.
        inbox: Vec<(String, Vec<u8>)>,
        /// Outbox files.
        outbox: Vec<(String, Vec<u8>)>,
        /// What the app keeps for itself across a lock: settings and the
        /// signed-amount memory.
        kept: Vec<(String, Vec<u8>)>,
    },
}

/// What the app asks the shell to do with storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageCommand {
    /// Read a file from a stick.
    Read {
        /// The stick's id.
        stick: String,
        /// The file.
        name: String,
    },
    /// Write a file to a stick: a new file, read back and compared; over
    /// a file of its name only when `replace`, which the person chose on
    /// the visit's Already on the stick sheet.
    Write {
        /// The stick's id.
        stick: String,
        /// The name to write under.
        name: String,
        /// The contents.
        bytes: Vec<u8>,
        /// Over the file of that name, rather than beside it.
        replace: bool,
        /// The person renamed the file For the stick and no visit has
        /// written it since: a vault going over its same vault takes this
        /// name, where otherwise it keeps the name it has on the stick
        /// (`docs/FLOWS.md` decision 61 item 2).
        renamed: bool,
    },
    /// Read the QR codes in a PNG on a stick. The shell decodes the image;
    /// the app sees only what the codes hold.
    ReadQr {
        /// The stick's id.
        stick: String,
        /// The image.
        name: String,
    },
    /// Delete a file from a stick, zeros written over it and read back
    /// first ([`StorageEvent::Wiped`] or [`StorageEvent::WipeFailed`]).
    Wipe {
        /// The stick's id.
        stick: String,
        /// The file.
        name: String,
    },
    /// A stick's free space and whole data space
    /// ([`StorageEvent::StickSpace`]).
    StickSpace {
        /// The stick's id.
        stick: String,
    },
    /// One step of zeros over a stick's free space, from cluster `from`
    /// ([`StorageEvent::Zeroing`] or [`StorageEvent::ZeroFailed`]).
    ZeroFree {
        /// The stick's id.
        stick: String,
        /// The cluster the step starts at.
        from: u32,
    },
    /// A stick's files partition made empty, its layout and label kept,
    /// before its clusters are zeroed ([`StorageEvent::Zeroing`]).
    MakeEmpty {
        /// The stick's id.
        stick: String,
    },
    /// Rename a file on a stick in place, refused where another file has
    /// the name ([`StorageEvent::Renamed`]; `docs/FLOWS.md` decision 61).
    Rename {
        /// The stick's id.
        stick: String,
        /// The file.
        from: String,
        /// Its new name.
        to: String,
    },
    /// A file on a stick read for its SHA-256, for its Details
    /// ([`StorageEvent::Hashed`]). The bytes go no further than the shell.
    Hash {
        /// The stick's id.
        stick: String,
        /// The file.
        name: String,
    },
    /// Save a PDF where the person prints from. Only an online shell (the
    /// desktop app) does; the device refuses.
    Print {
        /// The file's name.
        name: String,
        /// The PDF.
        bytes: Vec<u8>,
    },
    /// Read a file of this computer's for Transfer: one in Downloads, or
    /// one dropped on the window. Only the online shell does.
    ReadHost {
        /// Its path, as [`StorageEvent::HostFiles`] or
        /// [`StorageEvent::Dropped`] gave it.
        path: String,
    },
    /// Save a file Transfer received into Downloads, under `name` or,
    /// when that is taken, `name (2).ext` and on: never over a file.
    /// Only the online shell does; the device refuses.
    SaveDownload {
        /// The name to save it under.
        name: String,
        /// The contents.
        bytes: Vec<u8>,
    },
    /// Open Downloads in this computer's file manager.
    OpenDownloads,
    /// Open nakamotoinstitute.org in this computer's browser.
    OpenInstitute,
    /// Have the boot copier read the boot partition that holds the
    /// running Faraday (`docs/history/PLAN.md` §5.5).
    BootRead,
    /// Have the boot copier write the source over this boot partition,
    /// read back and compare. The app names the partition; it never hands
    /// the copier bytes.
    BootWrite {
        /// The partition, as [`BootPart::id`] named it.
        target: String,
    },
    /// The upgrade has ended: the copier drops the source.
    BootForget,
    /// Keep the Inbox and Outbox where the next process finds them. They
    /// hold nothing secret (`docs/history/PLAN.md` §5.2).
    SaveBoxes {
        /// Inbox files.
        inbox: Vec<(String, Vec<u8>)>,
        /// Outbox files.
        outbox: Vec<(String, Vec<u8>)>,
        /// What the app keeps for itself: settings and the signed-amount
        /// memory, nothing secret (`docs/WALLETS.md` §3.3).
        kept: Vec<(String, Vec<u8>)>,
    },
}

/// The disk process's answer to a request on a stick it does not hand out
/// to a process that is no longer clean: `faraday_files::proto::NOT_OPEN`,
/// which the app does not link; the visit's tests send that one.
const NOT_OPEN: &str = "stick-not-open";

impl StorageEvent {
    /// The event with the disk process's refusal of a process that is no
    /// longer clean ([`NOT_OPEN`]) in the app's words; any other reason as
    /// the disk gave it.
    pub(crate) fn in_own_words(self) -> Self {
        let said = |reason: String| {
            if reason == NOT_OPEN {
                crate::strings::EN.storage_event_not_open.to_string()
            } else {
                reason
            }
        };
        match self {
            Self::ReadFailed {
                stick,
                name,
                reason,
            } => Self::ReadFailed {
                stick,
                name,
                reason: said(reason),
            },
            Self::WriteFailed {
                stick,
                name,
                reason,
            } => Self::WriteFailed {
                stick,
                name,
                reason: said(reason),
            },
            Self::WipeFailed {
                stick,
                name,
                reason,
            } => Self::WipeFailed {
                stick,
                name,
                reason: said(reason),
            },
            Self::ZeroFailed { stick, reason } => Self::ZeroFailed {
                stick,
                reason: said(reason),
            },
            Self::Hashed { stick, name, sha } => Self::Hashed {
                stick,
                name,
                sha: sha.map_err(said),
            },
            Self::Renamed {
                stick,
                from,
                to,
                refused: Some(Unrenamed::Failed(why)),
            } => Self::Renamed {
                stick,
                from,
                to,
                refused: Some(Unrenamed::Failed(said(why))),
            },
            other => other,
        }
    }
}
