//! The boundary between the Starter's state and its shell
//! (`faraday/shells/Starter`, `docs/STARTER.md` §2): what the shell tells the Starter
//! ([`StarterEvent`]) and what the Starter asks the shell to do
//! ([`StarterCommand`]). Everything here is plain data; the Starter does no I/O.
//!
//! The shell sends what it reads, as it reads it, and sends it again
//! whenever it changes: this computer's facts at start, every disk
//! attached each time the set or a disk changes, the payload's manifest
//! once. The Starter keeps only the latest of each and keeps no file
//! (§5.1): Home's checklist is drawn from them every time.

/// Which operating system the Starter runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Os {
    /// Linux.
    #[default]
    Linux,
    /// Windows.
    Windows,
    /// macOS (`docs/MAC.md` §6).
    MacOs,
}

/// What the Starter reads about this computer (§7.1). Read locally and
/// shown only here.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Computer {
    /// The operating system.
    pub os: Os,
    /// The maker (`sys_vendor`, `SystemManufacturer`); empty when it
    /// could not be read.
    pub maker: String,
    /// The model (`product_name`, `SystemProductName`).
    pub model: String,
    /// The family (`product_family`, `SystemFamily`).
    pub family: String,
    /// It started in UEFI mode, not legacy BIOS.
    pub uefi: bool,
    /// It is an ARM computer, which cannot start the x86_64 stick.
    pub arm: bool,
    /// Secure Boot is on now; `None` when it could not be read.
    pub secure_boot: Option<bool>,
    /// Linux: a disk is unlocked by the TPM (`tpm2-device` in
    /// `/etc/crypttab`), so its passphrase is asked for after Secure
    /// Boot changes (§6.4).
    pub tpm_unlock: bool,
}

/// How many times Windows starts before a suspension the Starter makes
/// ends by itself (§6, F49): `DisableKeyProtectors`' `DisableCount`, as
/// `manage-bde -protectors -disable C: -RebootCount 3` sets it. Windows
/// keeps the count and lowers it each time Windows starts; the BIOS and
/// the stick do not run Windows, so they do not count (to be confirmed
/// on the Windows test virtual machine).
pub const SUSPEND_STARTS: u32 = 3;

/// The protection of the Windows system drive (§6, F49).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    /// Not encrypted.
    None,
    /// Encrypted and protected.
    On,
    /// Encrypted, protection suspended.
    Suspended,
    /// Could not be read.
    Unknown,
}

/// BitLocker on this computer's system drive (Windows only, §6, F49),
/// read from Windows each time the Start page is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitLocker {
    /// The system drive, as Windows names it (`C:`).
    pub drive: String,
    /// Its protection now.
    pub protection: Protection,
    /// While suspended, how many more times Windows starts before
    /// protection comes back by itself (0: not until resumed), when it
    /// was read: the elevated copy reads it (`GetSuspendCount`); the
    /// reading without elevation cannot.
    pub starts_left: Option<u32>,
}

/// A BitLocker recovery key, eight groups of six digits, with the drive
/// it opens (§3.4, F49): read by the elevated copy for the sheet, and
/// held only until the sheet is made. `Debug` prints none of the key,
/// and it is wiped when dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct RecoveryKey {
    drive: String,
    key: zeroize::Zeroizing<String>,
}

/// The groups in a recovery key.
pub const KEY_GROUPS: usize = 8;
/// The digits in a group.
pub const KEY_DIGITS: usize = 6;

/// Whether `s` is a drive, `C:`.
pub fn drive_ok(s: &str) -> bool {
    let d = s.as_bytes();
    d.len() == 2 && d[0].is_ascii_alphabetic() && d[1] == b':'
}

impl RecoveryKey {
    /// The key in `key` (groups separated by `-`, as Windows writes it)
    /// for `drive` (`C:`), or `None` for anything else.
    pub fn parse(drive: &str, key: &str) -> Option<RecoveryKey> {
        let key = key.trim();
        let key_ok = key.len() == KEY_GROUPS * KEY_DIGITS + KEY_GROUPS - 1
            && key.split('-').count() == KEY_GROUPS
            && key
                .split('-')
                .all(|g| g.len() == KEY_DIGITS && g.bytes().all(|b| b.is_ascii_digit()));
        if !drive_ok(drive) || !key_ok {
            return None;
        }
        let mut k = zeroize::Zeroizing::new(String::with_capacity(key.len()));
        k.push_str(key);
        Some(RecoveryKey {
            drive: drive.to_ascii_uppercase(),
            key: k,
        })
    }

    /// The drive (`C:`).
    pub fn drive(&self) -> &str {
        &self.drive
    }

    /// The key as Windows writes it.
    pub fn as_str(&self) -> &str {
        &self.key
    }
}

impl std::fmt::Debug for RecoveryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RecoveryKey({}, ..)", self.drive)
    }
}

/// A BitLocker step, done by the elevated copy (`--bitlocker <verb>`,
/// §2, §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitLockerStep {
    /// Suspend protection on the system drive for the next
    /// [`SUSPEND_STARTS`] times Windows starts.
    Suspend,
    /// Turn protection back on now.
    Resume,
}

/// How a disk is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bus {
    /// USB (udisks2's `Drive.ConnectionBus` = `usb`; Windows bus type
    /// USB). A stick, or an SD card in a USB reader.
    Usb,
    /// The computer's own SD card reader (Linux `mmcblk*`; Windows bus
    /// types SD and MMC).
    SdReader,
    /// Anything else (SATA, NVMe): never listed.
    Other,
}

/// Why the computer uses a disk (§4.2): such a disk is listed dimmed
/// and cannot be chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InUse {
    /// A partition on it is mounted somewhere other than `/run/media`
    /// or `/media` (`/mnt` and the rest are the computer's own places):
    /// the mount point.
    Mounted(String),
    /// A partition on it is active swap.
    Swap,
    /// Linux: the kernel lists another device built on the disk or one of
    /// its partitions (an LVM physical volume, a RAID member, an opened
    /// encrypted volume).
    Held,
    /// Linux: what is built on the disk or its partitions could not be
    /// read, so the disk is not offered.
    Unchecked,
    /// Windows: it holds the Windows, boot or page-file volume.
    System,
}

/// A volume on a disk, as the disk list describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    /// Its label, when it has one.
    pub label: Option<String>,
    /// It holds files.
    pub files: bool,
}

/// What a Faraday stick's boot partition says (§4.1): the stick has
/// FARADAYBOOT and FARADAYDATA, and FARADAYBOOT was read, read-only. FARADAYDATA is
/// never opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaradayStick {
    /// The kernel release string found on FARADAYBOOT
    /// (`faraday_boot::find_release`), or `None` for a stick made before
    /// Faraday put one there.
    pub release: Option<String>,
    /// FARADAYBOOT was read. On Windows it is not until keep data asks the
    /// elevated copy ([`StarterCommand::ReadVersion`]): FARADAYBOOT is an
    /// EFI system partition, which Windows lets only an administrator
    /// read. While `false`, `release` is `None` and says
    /// nothing about the stick.
    pub read: bool,
}

/// One disk attached, as the shell reads it (udisks2's `Drive`, `Block`,
/// `Partition` and `Filesystem` objects on Linux, §2; the storage
/// IOCTLs on Windows, §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    /// The shell's handle for it (a udisks2 object path,
    /// `\\.\PhysicalDriveN`); handed back unchanged in a
    /// [`StarterCommand::Write`].
    pub id: String,
    /// The device a person can check (`/dev/sdb`), shown on Confirm.
    pub device: String,
    /// The maker.
    pub vendor: String,
    /// The model.
    pub model: String,
    /// The serial number; empty when the disk reports none.
    pub serial: String,
    /// Its size in bytes.
    pub size: u64,
    /// How it is attached.
    pub bus: Bus,
    /// Its volumes.
    pub volumes: Vec<Volume>,
    /// Why the computer uses it, when it does.
    pub in_use: Option<InUse>,
    /// It is a Faraday stick (or card), and what its boot partition says.
    pub faraday: Option<FaradayStick>,
}

/// What a payload file holds, from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadKind {
    /// The stick image for a PC or an Intel Mac (`faraday-stick-…`).
    Stick,
    /// The stick image for a Mac with M1 or M2
    /// (`faraday-stick-…-arm64-apple.img.xz`, `docs/MAC.md` §4.1).
    AppleStick,
    /// The Pi image (`faraday-pi-…`).
    Pi,
}

impl PayloadKind {
    /// The kind a payload file's name says, or `None` for a name the Starter
    /// does not know.
    pub fn of(name: &str) -> Option<PayloadKind> {
        if name.starts_with("faraday-stick-") && name.ends_with("-arm64-apple.img.xz") {
            Some(PayloadKind::AppleStick)
        } else if name.starts_with("faraday-stick-") {
            Some(PayloadKind::Stick)
        } else if name.starts_with("faraday-pi-") {
            Some(PayloadKind::Pi)
        } else {
            None
        }
    }
}

/// One line of the payload's `MANIFEST` (§1.3), its tab-separated fields
/// as `faraday-payload` parses them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadFile {
    /// The payload file's name (`faraday-stick-0.2.0-x86_64-uefi.img.xz`).
    pub name: String,
    /// Its SHA-256, hex.
    pub sha256: String,
    /// The name of what is inside it (`faraday-stick-0.2.0-x86_64-uefi.img`).
    pub inner: String,
    /// That file's SHA-256, hex: what is written and checked.
    pub inner_sha256: String,
    /// That file's size in bytes.
    pub inner_size: u64,
    /// The build ID it carries (`docs/DECISIONS.md` F5).
    pub build_id: String,
}

impl PayloadFile {
    /// What it holds, from its name.
    pub fn kind(&self) -> Option<PayloadKind> {
        PayloadKind::of(&self.name)
    }
}

/// The payload the Starter carries (§1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// The release it is (`0.2.0`), from the manifest's first line.
    pub version: String,
    /// The commit it was built from, from the same line.
    pub commit: String,
    /// Its files.
    pub files: Vec<PayloadFile>,
}

/// A developer's own image (`faraday-starter --image FILE`, §1.1), in place
/// of the payload's stick image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevImage {
    /// The file as given.
    pub path: String,
    /// The SHA-256 of what is written (the file, decompressed when it is
    /// a `.img.xz`), hex.
    pub sha256: String,
    /// The size of what is written, in bytes.
    pub size: u64,
    /// It is a boot partition alone (`-boot.vfat`): only *keep data* can
    /// write it.
    pub boot_only: bool,
    /// The build ID found in it, when one was.
    pub build_id: Option<String>,
}

/// One thing an install put on this computer (§5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallItem {
    /// The Starter.
    App,
    /// The Starter's menu entry or Start menu shortcut.
    Menu,
    /// The icon.
    Icon,
    /// Windows: the entry in Installed apps.
    UninstallEntry,
}

/// What is installed (§5.2), as the shell finds it on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// The installed Starter's version.
    pub version: String,
    /// Each thing installed and where.
    pub items: Vec<(InstallItem, String)>,
}

/// How far a write is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteStage {
    /// Writing: bytes written of the total.
    Writing,
    /// Reading the write back to compare: bytes read of the total.
    ReadingBack,
}

/// Why a partition table is not a Faraday stick's (§4.1), in
/// `faraday/image/stick-boot.sh`'s terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StickFault {
    /// The table could not be read: the system's words.
    Reading(String),
    /// No GPT header at all.
    NoHeader,
    /// The table is not GPT.
    NotGpt,
    /// The GPT header is damaged.
    HeaderDamaged,
    /// The GPT header's checksum is wrong.
    HeaderChecksum,
    /// The GPT entries are damaged.
    EntriesDamaged,
    /// The GPT entries lie past the end of the disk.
    EntriesPastEnd,
    /// The GPT entries' checksum is wrong.
    EntriesChecksum,
    /// The disk UUID is not a Faraday stick's.
    DiskUuid,
    /// The number of partitions is not two.
    Partitions(usize),
    /// The first partition is not named FARADAYBOOT.
    BootName,
    /// The first partition is not a Faraday EFI system partition.
    BootType,
    /// The second partition is not a Faraday FARADAYDATA partition.
    DataType,
    /// The FARADAYBOOT partition is damaged.
    BootDamaged,
    /// The FARADAYBOOT partition is smaller than the image.
    BootSmall {
        /// The partition's size in bytes.
        boot: u64,
        /// The image's.
        image: u64,
    },
    /// The partitions overlap.
    Overlap,
    /// The image's boot partition runs past the image's end.
    BootPastEnd,
}

/// Why something the shell was asked to do did not happen: the one list
/// the screens turn into `starter_` strings (`faraday_strings`). Where a
/// reason carries text (`detail`), it is the operating system's own
/// words for what failed, shown after the Starter's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// This computer cannot write a disk from here.
    NoWriter,
    /// The Starter does not carry what was confirmed.
    NotCarried,
    /// The disk is not the one confirmed.
    Changed {
        /// The device confirmed.
        device: String,
        /// The serial it has now; empty for none.
        serial: String,
        /// The size it has now, bytes.
        size: u64,
    },
    /// The computer uses the disk.
    InUse(InUse),
    /// A boot partition alone was ordered over a whole disk.
    BootOnly,
    /// The disk is smaller than the image.
    TooSmall {
        /// The disk's size, bytes.
        disk: u64,
        /// The image's.
        image: u64,
    },
    /// The payload file's SHA-256 is not the manifest's.
    PayloadSum,
    /// A developer's image changed since the Starter started.
    ImageChanged {
        /// The file.
        path: String,
        /// Its SHA-256 now.
        sha256: String,
    },
    /// The disk is no longer attached.
    Gone,
    /// The system bus (udisks2) said no.
    Bus(String),
    /// Unmounting what the desktop mounted failed.
    Unmounting(String),
    /// Opening the disk to write failed.
    OpenWrite(String),
    /// Opening the disk to read back failed.
    OpenRead(String),
    /// Reaching the disk failed.
    Device {
        /// The device.
        device: String,
        /// The system's words.
        detail: String,
    },
    /// Flushing the disk failed.
    Flush {
        /// The device.
        device: String,
        /// The system's words.
        detail: String,
    },
    /// The disk is not a Faraday stick (keep data).
    NotStick {
        /// The device.
        device: String,
        /// Why.
        fault: StickFault,
    },
    /// The image is not a Faraday stick image.
    NotStickImage(StickFault),
    /// The image's boot partition is larger than the stick's.
    BootLarger {
        /// The image's boot partition, bytes.
        image: u64,
        /// The stick's.
        stick: u64,
    },
    /// The image's boot partition is not FAT32 labelled FARADAYBOOT.
    BootLabel,
    /// The boot partition image is shorter than one sector.
    BootShort,
    /// The partition table changed during the write.
    TableChanged(String),
    /// Reading the partition table failed.
    ReadingTable(String),
    /// Reading back: the disk ends early.
    BackEnds {
        /// The device.
        device: String,
        /// The byte it ends at.
        at: u64,
    },
    /// Reading back: the disk differs from what was written.
    BackDiffers {
        /// The device.
        device: String,
        /// The first byte that differs.
        at: u64,
    },
    /// Reading back failed.
    BackFailed {
        /// The device.
        device: String,
        /// The system's words.
        detail: String,
    },
    /// Reading the source failed.
    SourceRead(String),
    /// Writing failed.
    WriteFailed(String),
    /// The payload file changed while it was read.
    PayloadChanged,
    /// The `.xz` is damaged.
    Damaged(String),
    /// The decompressed image is longer than expected.
    Longer(u64),
    /// The decompressed image is shorter than expected.
    Shorter {
        /// The expected size.
        expected: u64,
        /// The actual size.
        actual: u64,
    },
    /// The decompressed image's SHA-256 is not the expected one.
    WrongSum(String),
    /// The home folder is not known.
    NoHome,
    /// A file or folder could not be written, read or removed.
    File {
        /// Where.
        path: String,
        /// The system's words.
        detail: String,
    },
    /// A file of the name install would write is another program's.
    InTheWay {
        /// Where it is, as a person reads it.
        path: String,
    },
    /// A program could not be started.
    Start {
        /// Which.
        path: String,
        /// The system's words.
        detail: String,
    },
    /// The restart into firmware settings failed: the system's words.
    Restart(String),
    /// A program the step needs is not on this computer: its name.
    Missing(String),
    /// `efibootmgr` failed while choosing the stick for the next start
    /// (§3.1): its words.
    BootEntry(String),
    /// No Faraday stick is attached.
    NoStick,
    /// Keep data: the stick already holds the build to be written, as
    /// the elevated writer read it on Windows.
    SameBuild,
    /// The person declined the administrator permission a step asks
    /// for (Windows' elevation prompt, polkit's password on Linux), so
    /// nothing was done.
    Declined,
    /// The elevation prompt was declined on an account that is not an
    /// administrator's: an administrator has to do the step.
    NoAdmin,
    /// BitLocker refused to suspend (`DisableKeyProtectors`' return
    /// value, or 0 when the volume was still protected after it): the
    /// computer's BitLocker is managed.
    Managed(u32),
    /// A BitLocker call failed: the method and its return value or
    /// HRESULT.
    BitLocker {
        /// The method (`GetKeyProtectors`).
        call: String,
        /// What it returned.
        code: u32,
    },
    /// The system drive is not encrypted.
    NotEncrypted,
    /// The system drive's protection could not be read: Windows did not
    /// list it, or answered something else.
    NotRead,
    /// The system drive has no recovery key (no numerical password).
    NoRecoveryKey,
    /// Not on this computer.
    Unsupported,
}

/// What an attached thing of concern is (§3.1a): something that can
/// capture what Faraday shows or what is typed without Faraday's help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A TV or other screen that records (its EDID).
    Tv,
    /// A KVM switch or IP-KVM (its USB identity, or its capture side's
    /// EDID name).
    Kvm,
    /// A wireless keyboard or mouse receiver: a line on Home, which
    /// holds nothing.
    Receiver,
    /// Logitech's Bolt receiver, whose link is encrypted: a line on Home
    /// that says so, in place of the receiver's.
    BoltReceiver,
    /// This computer is a virtual machine: the stick must start on the
    /// real computer, and the Starter cannot see the host's devices.
    VirtualMachine,
}

/// One thing attached that is of concern (§3.1a), as the shell reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concern {
    /// What it is.
    pub kind: Kind,
    /// Its name as the system reads it (the EDID's monitor name, the USB
    /// maker and product strings); empty when it has none.
    pub name: String,
    /// The shell's handle for it, the same while it stays attached (the
    /// connector, the USB port, `vm`): what Continue anyway lets through
    /// for the session.
    pub id: String,
}

/// What the shell tells the Starter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StarterEvent {
    /// This computer's facts (§7.1). Sent at start.
    Computer(Computer),
    /// Every disk attached now. Sent at start and whenever the set or a
    /// disk changes (plugged in, pulled out, mounted, written).
    Disks(Vec<Disk>),
    /// The payload's manifest, or `None` for a Starter built without one.
    Payload(Option<Payload>),
    /// A developer's image given with `--image`.
    DevImage(DevImage),
    /// What is installed, or `None` when nothing is. Sent at start and
    /// after an install or uninstall.
    Installed(Option<Installed>),
    /// This computer cannot write a disk from here (Linux without
    /// udisks2, §2).
    NoWriter,
    /// A write's progress: bytes done of the total.
    WriteProgress {
        /// Writing or reading back.
        stage: WriteStage,
        /// Bytes done.
        done: u64,
        /// Bytes in all.
        total: u64,
    },
    /// The write ended: read back and matched, or the writer's reason.
    Written(Result<(), Reason>),
    /// The install ended: where the Starter went, or why not.
    InstallDone(Result<String, Reason>),
    /// The uninstall ended.
    UninstallDone(Result<(), Reason>),
    /// Faraday Demo opened in the window (F126, §5.3), or why not: its
    /// stick folders could not be made.
    DesktopStarted(Result<(), Reason>),
    /// The restart into firmware settings was refused or failed, with
    /// why (a restart that happens sends nothing).
    RestartFailed(Reason),
    /// The restart from the stick was refused or failed, with why; the
    /// card offers the restart into firmware settings instead (§3.1).
    StickRestartFailed(Reason),
    /// Uninstall left the firmware's `Faraday stick` boot entry in
    /// place (Linux, §3.1), with why. Sent before
    /// [`StarterEvent::UninstallDone`].
    BootEntryKept(Reason),
    /// Another encryption product's driver is on this computer (§6.4):
    /// its name ("VeraCrypt"), or `None`. Sent at start.
    OtherEncryption(Option<String>),
    /// What is attached that is of concern (§3.1a), all of it. Sent at
    /// start, whenever it changes, and after [`StarterCommand::CheckAgain`].
    Concerns(Vec<Concern>),
    /// Whether the firmware's `Faraday stick` boot entry exists (Linux,
    /// §3.1c), from the unprivileged `efibootmgr` listing. Sent at start
    /// and after the entry is made or removed.
    BootEntry(bool),
    /// [`StarterCommand::RemoveBootEntry`] ended: removed, or why not.
    BootEntryRemoved(Result<(), Reason>),
    /// [`StarterCommand::SaveTemplate`] ended: the path it was saved at,
    /// or why not.
    TemplateSaved(Result<String, Reason>),
    /// [`StarterCommand::PrintTemplate`] or [`StarterCommand::OpenTemplate`]
    /// ended: opened in the viewer, or why not.
    TemplateOpened(Result<(), Reason>),
    /// [`StarterCommand::PrintNotes`] ended: opened in the viewer, or why
    /// not.
    NotesOpened(Result<(), Reason>),
    /// BitLocker on the system drive (Windows, §6, F49), as read after
    /// [`StarterCommand::ReadBitLocker`] and after each BitLocker step;
    /// `None` where there is none to read.
    BitLocker(Option<BitLocker>),
    /// [`StarterCommand::BitLocker`] ended: done, or why not. The shell
    /// sends [`StarterEvent::BitLocker`] after it, as read then.
    BitLockerDone {
        /// Which step.
        step: BitLockerStep,
        /// Done, or why not.
        result: Result<(), Reason>,
    },
    /// [`StarterCommand::ReadRecoveryKey`] ended: the key, or why not.
    RecoveryKey(Result<RecoveryKey, Reason>),
    /// [`StarterCommand::ReadVersion`] ended: the release string on the
    /// stick's FARADAYBOOT (`None` for a stick made before Faraday put one
    /// there), or why not.
    VersionRead {
        /// The disk, as the command named it.
        disk: String,
        /// Its serial, as the command named it.
        serial: String,
        /// Its size, as the command named it.
        size: u64,
        /// What was read, or why not.
        result: Result<Option<String>, Reason>,
    },
}

/// How a stick or card is written (§3.2 step 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The whole disk; everything on it is lost.
    Whole,
    /// Only the boot partition of a Faraday stick; FARADAYDATA is not read
    /// and not written.
    KeepData,
}

/// What a write takes its bytes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A payload file, by its name in the manifest; the writer checks
    /// the decompressed stream against `inner_sha256`.
    Payload {
        /// The payload file's name.
        name: String,
        /// The SHA-256 of what is inside it, hex.
        inner_sha256: String,
    },
    /// A developer's image (`--image`), checked against `sha256`.
    Image {
        /// The file as given.
        path: String,
        /// Its SHA-256, hex.
        sha256: String,
    },
}

/// A confirmed write: everything the writer checks again before it
/// writes (§4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOrder {
    /// The bytes.
    pub source: Source,
    /// The disk's handle, from [`Disk::id`].
    pub disk: String,
    /// Its device, as confirmed.
    pub device: String,
    /// Its serial, as confirmed: the writer refuses another.
    pub serial: String,
    /// Its size in bytes, as confirmed: the writer refuses another.
    pub size: u64,
    /// Whole or keep data.
    pub mode: Mode,
}

/// What the Starter asks the shell to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StarterCommand {
    /// Write a stick or card as confirmed.
    Write(WriteOrder),
    /// Install the Starter (§5.2), removing an earlier install's
    /// Faraday Demo (§5.3).
    Install,
    /// Uninstall exactly what install put here (§5.4).
    Uninstall,
    /// Open Faraday Demo in the window, under the top app bar (F126,
    /// §5.3): the shell makes its app and hands it to
    /// [`crate::Starter::show_demo`].
    OpenDemo,
    /// Faraday Demo was left: the shell lets go of what it kept for it.
    DemoClosed,
    /// Restart into firmware settings (§7.3).
    RestartIntoFirmware,
    /// Restart from the stick (§3.1): Windows' advanced startup, whose
    /// *Use a device* starts it; on Linux the attached stick once, at
    /// the next start.
    RestartFromStick {
        /// The attached Faraday stick's device (`/dev/sdb`), from
        /// [`Disk::device`].
        device: String,
    },
    /// Read what is attached again now (§3.1a, Check again), and send
    /// [`StarterEvent::Concerns`].
    CheckAgain,
    /// Remove the firmware's `Faraday stick` boot entry (Linux, §3.1c),
    /// through one polkit prompt.
    RemoveBootEntry,
    /// Save a blank template's PDF into the Downloads folder (§3.8),
    /// never over a file; answered by [`StarterEvent::TemplateSaved`].
    SaveTemplate {
        /// The file's name.
        name: String,
        /// The PDF.
        pdf: Vec<u8>,
    },
    /// Print a blank template (§3.8): write the PDF into the system's
    /// temporary folder and open it in the system's PDF viewer, which
    /// prints to any printer; answered by [`StarterEvent::TemplateOpened`].
    PrintTemplate {
        /// The file's name.
        name: String,
        /// The PDF.
        pdf: Vec<u8>,
    },
    /// Open a file in the system's PDF viewer (§3.8, Open it); answered
    /// by [`StarterEvent::TemplateOpened`].
    OpenTemplate {
        /// The file's path, as [`StarterEvent::TemplateSaved`] gave it.
        path: String,
    },
    /// Print what to write down before the restart (§3.4, F47): write
    /// the PDF into the system's temporary folder and open it in the
    /// system's viewer, where the person prints or saves it, as
    /// [`StarterCommand::PrintTemplate`] does; answered by
    /// [`StarterEvent::NotesOpened`].
    PrintNotes {
        /// The file's name.
        name: String,
        /// The PDF.
        pdf: Vec<u8>,
        /// It carries the BitLocker recovery key (F49): the shell wipes
        /// its copy once written, and removes the file when the Starter
        /// exits.
        with_key: bool,
    },
    /// Read BitLocker on the system drive (Windows, §6, F49), without
    /// elevation; answered by [`StarterEvent::BitLocker`]. Asked each
    /// time the Start page is shown.
    ReadBitLocker,
    /// A BitLocker step through the elevated copy; answered by
    /// [`StarterEvent::BitLockerDone`], then [`StarterEvent::BitLocker`].
    BitLocker(BitLockerStep),
    /// Read the system drive's recovery key through the elevated copy,
    /// for the sheet (F49); answered by [`StarterEvent::RecoveryKey`].
    ReadRecoveryKey,
    /// Read a Faraday stick's version through the elevated copy
    /// (Windows), for keep data's rules; answered by
    /// [`StarterEvent::VersionRead`]. The elevated copy refuses a disk
    /// whose serial or size is not these.
    ReadVersion {
        /// The disk's handle, from [`Disk::id`].
        disk: String,
        /// Its serial.
        serial: String,
        /// Its size in bytes.
        size: u64,
    },
}
