//! The boundary between the hub app's state and its shell
//! (`faraday/shells/hub`, `docs/HUB.md` §2): what the shell tells the hub
//! ([`HubEvent`]) and what the hub asks the shell to do
//! ([`HubCommand`]). Everything here is plain data; the hub does no I/O.
//!
//! The shell sends what it reads, as it reads it, and sends it again
//! whenever it changes: this computer's facts at start, every disk
//! attached each time the set or a disk changes, the payload's manifest
//! once. The hub keeps only the latest of each and keeps no file
//! (§5.1): Home's checklist is drawn from them every time.

/// Which operating system the hub app runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Os {
    /// Linux.
    #[default]
    Linux,
    /// Windows.
    Windows,
}

/// What the hub app reads about this computer (§7.1). Read locally and
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

/// The protection of the Windows system drive, as Home reads it without
/// elevation (`System.Volume.BitLockerProtection`, §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    /// Not encrypted: the guard is not needed.
    None,
    /// Encrypted and protected.
    On,
    /// Encrypted, protection suspended.
    Suspended,
    /// Could not be read: treated as protected.
    Unknown,
}

/// The hub app's marker (§6.3), when it exists: it suspended BitLocker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    /// The volumes it suspended ("C:").
    pub volumes: Vec<String>,
    /// Whether Secure Boot was on when it suspended.
    pub secure_boot_was_on: bool,
    /// The marker was written in the Windows session now running: the
    /// person who suspended is still in it, and Resume would undo the
    /// step just done (§6.3). A marker with no boot recorded is from an
    /// earlier boot.
    pub this_boot: bool,
}

/// BitLocker on this computer (Windows only, §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitLocker {
    /// The system drive, as Windows names it ("C:").
    pub drive: String,
    /// Its protection now.
    pub protection: Protection,
    /// The hub app's marker, when it exists.
    pub marker: Option<Marker>,
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
    /// A partition on it is mounted somewhere other than `/run/media`,
    /// `/media` or `/mnt`: the mount point.
    Mounted(String),
    /// A partition on it is active swap.
    Swap,
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
/// OSKBOOT and OSKDATA, and OSKBOOT was read through `faraday-fat`,
/// read-only. OSKDATA is never opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaradayStick {
    /// The kernel release string found on OSKBOOT
    /// (`faraday_boot::find_release`), or `None` for a stick made before
    /// Faraday put one there.
    pub release: Option<String>,
}

/// One disk attached, as the shell reads it (udisks2's `Drive`, `Block`,
/// `Partition` and `Filesystem` objects on Linux, §2; the storage
/// IOCTLs on Windows, §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    /// The shell's handle for it (a udisks2 object path,
    /// `\\.\PhysicalDriveN`); handed back unchanged in a
    /// [`HubCommand::Write`].
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
    /// The stick image (`faraday-stick-…`).
    Stick,
    /// The Pi image (`faraday-pi-…`).
    Pi,
    /// The desktop app (`faraday-desktop-…`).
    Desktop,
}

impl PayloadKind {
    /// The kind a payload file's name says, or `None` for a name the hub
    /// does not know.
    pub fn of(name: &str) -> Option<PayloadKind> {
        if name.starts_with("faraday-stick-") {
            Some(PayloadKind::Stick)
        } else if name.starts_with("faraday-pi-") {
            Some(PayloadKind::Pi)
        } else if name.starts_with("faraday-desktop-") {
            Some(PayloadKind::Desktop)
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

/// The payload the hub app carries (§1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// The release it is (`0.2.0`), from the manifest's first line.
    pub version: String,
    /// The commit it was built from, from the same line.
    pub commit: String,
    /// Its files.
    pub files: Vec<PayloadFile>,
}

/// A developer's own image (`faraday-hub --image FILE`, §1.1), in place
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
    /// The hub app.
    App,
    /// The desktop app.
    Desktop,
    /// The hub app's menu entry or Start menu shortcut.
    Menu,
    /// The desktop app's menu entry or shortcut.
    DesktopMenu,
    /// The icon.
    Icon,
    /// Windows: the entry in Installed apps.
    UninstallEntry,
    /// Windows: the logon reminder (§6.3).
    Reminder,
}

/// What is installed (§5.2), as the shell finds it on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// The installed hub app's version.
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
    /// The first partition is not named OSKBOOT.
    BootName,
    /// The first partition is not a Faraday EFI system partition.
    BootType,
    /// The second partition is not a Faraday OSKDATA partition.
    DataType,
    /// The OSKBOOT partition is damaged.
    BootDamaged,
    /// The OSKBOOT partition is smaller than the image.
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
/// the screens turn into `hub_` strings (`faraday_core::strings`). Where a
/// reason carries text (`detail`), it is the operating system's own
/// words for what failed, shown after the hub's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// This computer cannot write a disk from here.
    NoWriter,
    /// The hub app does not carry what was confirmed.
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
    /// A developer's image changed since the hub app started.
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
    /// The image's boot partition is not FAT32 labelled OSKBOOT.
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
    /// The payload carries no desktop app.
    NoDesktopApp,
    /// The cached desktop app does not match the manifest.
    CacheSum(String),
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
    /// The person declined the administrator permission a step asks
    /// for (Windows' elevation prompt, polkit's password on Linux), so
    /// nothing was done.
    Declined,
    /// The elevation prompt was declined on an account that is not an
    /// administrator's (§6.4): an administrator has to do the step.
    NoAdmin,
    /// BitLocker refused to suspend (`DisableKeyProtectors`' return
    /// value, or 0 when the volume was still protected after it): the
    /// computer's BitLocker is managed (§6.4).
    Managed(u32),
    /// A BitLocker call failed: the method and its return value or
    /// HRESULT.
    BitLocker {
        /// The method (`GetKeyProtectors`).
        call: String,
        /// What it returned.
        code: u32,
    },
    /// The system drive is not encrypted: there is nothing to suspend.
    NotEncrypted,
    /// The system drive's protection could not be read: Windows did not
    /// list it, or answered something else. It is not known, which the
    /// guard treats as protected (§6.2).
    NotRead,
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
    /// A wireless keyboard or mouse receiver.
    Receiver,
    /// This computer is a virtual machine: the stick must start on the
    /// real computer, and the hub app cannot see the host's devices.
    VirtualMachine,
    /// A remote session (Windows `SM_REMOTESESSION`, X11 over the
    /// network). Held only around the recovery key.
    RemoteSession,
    /// A wireless display (Miracast). Held only around the recovery key.
    WirelessDisplay,
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

/// What the shell tells the hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HubEvent {
    /// This computer's facts (§7.1). Sent at start.
    Computer(Computer),
    /// BitLocker on the system drive (Windows), sent at start and after
    /// each BitLocker step; `None` on Linux.
    BitLocker(Option<BitLocker>),
    /// Every disk attached now. Sent at start and whenever the set or a
    /// disk changes (plugged in, pulled out, mounted, written).
    Disks(Vec<Disk>),
    /// The payload's manifest, or `None` for a hub app built without one.
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
    /// The install ended: where the hub app went, or why not.
    InstallDone(Result<String, Reason>),
    /// The uninstall ended.
    UninstallDone(Result<(), Reason>),
    /// The desktop app was started, or why not.
    DesktopStarted(Result<(), Reason>),
    /// The restart into firmware settings was refused or failed, with
    /// why (a restart that happens sends nothing).
    RestartFailed(Reason),
    /// The restart from the stick was refused or failed, with why; the
    /// card offers the restart into firmware settings instead (§3.1).
    StickRestartFailed(Reason),
    /// Uninstall left the firmware's `Faraday stick` boot entry in
    /// place (Linux, §3.1), with why. Sent before
    /// [`HubEvent::UninstallDone`].
    BootEntryKept(Reason),
    /// Another encryption product's driver is on this computer (§6.4):
    /// its name ("VeraCrypt"), or `None`. Sent at start.
    OtherEncryption(Option<String>),
    /// [`BitLockerStep::ShowKey`] ended: the key, or why not. The shell
    /// sends [`HubEvent::BitLocker`] after it, as read then.
    KeyShown(Result<KeyShown, Reason>),
    /// [`BitLockerStep::Suspend`] or [`BitLockerStep::Resume`] ended. The
    /// shell sends [`HubEvent::BitLocker`] after it, as read then.
    BitLockerDone {
        /// Which step.
        step: BitLockerStep,
        /// Done, or why not.
        result: Result<(), Reason>,
    },
    /// [`HubCommand::SaveKey`] ended: where the file went, or why not.
    KeySaved(Result<String, Reason>),
    /// What is attached that is of concern (§3.1a), all of it. Sent at
    /// start, whenever it changes, and after [`HubCommand::CheckAgain`].
    Concerns(Vec<Concern>),
    /// Whether the firmware's `Faraday stick` boot entry exists (Linux,
    /// §3.1c), from the unprivileged `efibootmgr` listing. Sent at start
    /// and after the entry is made or removed.
    BootEntry(bool),
    /// [`HubCommand::RemoveBootEntry`] ended: removed, or why not.
    BootEntryRemoved(Result<(), Reason>),
}

/// The recovery key the elevated copy read (§6.2), for the guard's page.
/// The key is wiped when this is dropped, and `Debug` prints none of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyShown {
    /// The drive, its computer and its key, as the QR code carries them.
    pub text: faraday_qr::bitlocker::KeyText,
    /// The key protector's ID (`{6A0E…}`), whose first eight digits
    /// Windows' recovery screen shows as the key ID.
    pub id: String,
}

impl KeyShown {
    /// The key ID Windows' recovery screen shows: the protector ID's
    /// first eight hex digits.
    pub fn key_id(&self) -> String {
        self.id
            .trim_start_matches('{')
            .chars()
            .take(8)
            .collect::<String>()
            .to_uppercase()
    }
}

/// How a stick or card is written (§3.2 step 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The whole disk; everything on it is lost.
    Whole,
    /// Only the boot partition of a Faraday stick; OSKDATA is not read
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

/// A BitLocker step, done by the elevated copy (`--bitlocker <verb>`,
/// §2, §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitLockerStep {
    /// Read the recovery key, adding one when the volume has none.
    ShowKey,
    /// Suspend protection until resumed, and write the marker.
    Suspend,
    /// Resume protection on the marker's volumes, and delete the marker.
    Resume,
}

/// What the hub asks the shell to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HubCommand {
    /// Write a stick or card as confirmed.
    Write(WriteOrder),
    /// Install the hub app, with the desktop app when `desktop`.
    Install {
        /// Also install the desktop app.
        desktop: bool,
    },
    /// Uninstall exactly what install put here (§5.4).
    Uninstall,
    /// Run the desktop app: the installed one, else from the payload
    /// (§5.3).
    RunDesktop,
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
    /// A BitLocker step through the elevated copy.
    BitLocker(BitLockerStep),
    /// Save the key shown to a file on `disk` (§6.2): only a removable
    /// disk that is not a Faraday stick, which the shell checks again.
    SaveKey {
        /// The disk's handle, from [`Disk::id`].
        disk: String,
        /// The key.
        key: KeyShown,
    },
    /// Read what is attached again now (§3.1a, Check again), and send
    /// [`HubEvent::Concerns`].
    CheckAgain,
    /// Remove the firmware's `Faraday stick` boot entry (Linux, §3.1c),
    /// through one polkit prompt.
    RemoveBootEntry,
}

/// What `--remind` does at logon (§6.3), decided by [`remind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remind {
    /// No marker, or protection already on: remove the Run entry and
    /// exit with no window.
    RemoveAndExit,
    /// The marker, and Secure Boot not as it recorded: exit with no
    /// window.
    Exit,
    /// The marker, and Secure Boot as it recorded: open the guard's page
    /// with Resume BitLocker as the one thing to do.
    Open,
}

/// `--remind`'s decision (§6.3), from the marker, the protection now and
/// Secure Boot now. Resume is offered while Secure Boot is as the marker
/// recorded it (on for a computer that had it on when the hub app
/// suspended BitLocker, off for one that already had it off); a state
/// that could not be read is not as recorded. A drive no longer
/// encrypted has nothing to resume.
pub fn remind(
    marker: Option<&Marker>,
    protection: Protection,
    secure_boot: Option<bool>,
) -> Remind {
    match marker {
        None => Remind::RemoveAndExit,
        Some(_) if matches!(protection, Protection::On | Protection::None) => Remind::RemoveAndExit,
        Some(m) if m.this_boot => Remind::Exit,
        Some(m) if secure_boot == Some(m.secure_boot_was_on) => Remind::Open,
        Some(_) => Remind::Exit,
    }
}
