//! The Starter's operating system: its asks, answers and value types (02 §4.10). `main`'s
//! `StarterCommand` and `StarterEvent` value types, with `String` as `Text`.
// Spec: 02 §4.10 says "typed as they are on main" and does not list the variants of
// `SystemAnswer`; each answer's name and payload here follows `main`'s `StarterEvent`.

use alloc::vec::Vec;

use faraday_text::{SecretText, Text};

use crate::ask::request;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Os {
    #[default]
    Linux,
    Windows,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Computer {
    pub os: Os,
    pub maker: Text,
    pub model: Text,
    pub family: Text,
    pub uefi: bool,
    pub arm: bool,
    pub secure_boot: Option<bool>,
    pub tpm_unlock: bool,
    pub camera: Camera,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Camera {
    #[default]
    Unknown,
    Usable,
    NotUsable,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protection {
    None,
    On,
    Suspended,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitLocker {
    pub drive: Text,
    pub protection: Protection,
    pub starts_left: Option<u32>,
}

/// A BitLocker recovery key. The app checks its shape.
// Spec: 02 §4.10 leaves this open (`main`'s has private fields and its own checks).
pub struct RecoveryKey {
    pub drive: Text,
    pub id: Text,
    pub key: SecretText,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitLockerStep {
    Suspend,
    Resume,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bus {
    Usb,
    SdReader,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InUse {
    Mounted(Text),
    Swap,
    Held,
    Unchecked,
    System,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    pub label: Option<Text>,
    pub files: bool,
    pub fs: Text,
    pub encrypted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaradayStick {
    pub release: Option<Text>,
    pub read: bool,
    pub boot_sha256: Option<Text>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disk {
    pub id: Text,
    pub device: Text,
    pub vendor: Text,
    pub model: Text,
    pub serial: Text,
    pub size: u64,
    pub bus: Bus,
    pub fixed: bool,
    pub volumes: Vec<Volume>,
    pub in_use: Option<InUse>,
    pub faraday: Option<FaradayStick>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevImage {
    pub path: Text,
    pub sha256: Text,
    pub size: u64,
    pub boot_only: bool,
    pub build_id: Option<Text>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallItem {
    App,
    Menu,
    Icon,
    UninstallEntry,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub version: Text,
    pub items: Vec<(InstallItem, Text)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteStage {
    Writing,
    ReadingBack,
    Clearing,
}

/// What is wrong with a stick or an image's layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StickFault {
    Reading(Text),
    NoHeader,
    NotGpt,
    HeaderDamaged,
    HeaderChecksum,
    EntriesDamaged,
    EntriesPastEnd,
    EntriesChecksum,
    Partitions(usize),
    BootType,
    DataType,
    BootDamaged,
    BootSmall { boot: u64, image: u64 },
    BootPlace,
    DataPlace,
    BootPastEnd,
    CardLayout,
    NoSerial,
    BackupTable,
    Name,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReleaseFault {
    Image,
    // Spec: 02 §4.10 leaves this open: `main`'s holds `faraday_fat::Error`, which the port
    // cannot name (P5).
    Fat(Text),
}

/// Why a system ask failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    NoWriter,
    NotCarried,
    Changed {
        device: Text,
        serial: Text,
        size: u64,
    },
    InUse(InUse),
    BootOnly,
    TooSmall {
        disk: u64,
        image: u64,
    },
    PayloadSum,
    ImageChanged {
        path: Text,
        sha256: Text,
    },
    Gone,
    Bus(Text),
    Unmounting(Text),
    OpenWrite(Text),
    OpenRead(Text),
    Device {
        device: Text,
        detail: Text,
    },
    Flush {
        device: Text,
        detail: Text,
    },
    NotStick {
        device: Text,
        fault: StickFault,
    },
    NotStickImage(StickFault),
    BootLarger {
        image: u64,
        stick: u64,
    },
    BootLabel,
    BootShort,
    SectorSize {
        device: Text,
        sector: u64,
    },
    TableChanged(Text),
    ReadingTable(Text),
    BackEnds {
        device: Text,
        at: u64,
    },
    BackDiffers {
        device: Text,
        at: u64,
    },
    BackFailed {
        device: Text,
        detail: Text,
    },
    SourceRead(Text),
    WriteFailed(Text),
    PayloadChanged,
    Damaged(Text),
    Longer(u64),
    Shorter {
        expected: u64,
        actual: u64,
    },
    WrongSum(Text),
    NoHome,
    File {
        path: Text,
        detail: Text,
    },
    InTheWay {
        path: Text,
    },
    Start {
        path: Text,
        detail: Text,
    },
    Restart(Text),
    Missing(Text),
    BootEntry(Text),
    NoStick,
    SameBuild,
    Declined,
    NoAdmin,
    Managed(u32),
    BitLocker {
        call: Text,
        code: u32,
    },
    NotEncrypted,
    NotRead,
    NoRecoveryKey,
    KeyFolder,
    BootFileOther,
    NotJudged,
    BootFileUnsigned,
    BootFileBroken,
    BootFileNotRead,
    BootFileLayout,
    BootFileRead(Text),
    BootFileChanged(Text),
    BootFileLarge(Text),
    ReleaseBootFile(ReleaseFault),
    Unsupported,
    Random(Text),
    BackSum {
        device: Text,
        sha256: Text,
    },
    BootSum {
        sha256: Text,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConcernKind {
    Tv,
    Kvm,
    Receiver,
    BoltReceiver,
    VirtualMachine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Concern {
    pub kind: ConcernKind,
    pub name: Text,
    pub id: Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageFinding {
    Found(FoundImage),
    Mismatch { path: Text, name: Text },
    OtherRelease { path: Text, name: Text, needs: Text },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundImage {
    pub path: Text,
    pub name: Text,
    pub xz_sha256: Text,
    pub img_sha256: Text,
    pub inner_size: u64,
    pub build_id: Text,
    pub board: Text,
    pub panel: Text,
    pub starts: Vec<Text>,
    pub boot_sha256: Option<Text>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Whole,
    KeepData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Found {
        path: Text,
        xz_sha256: Text,
        img_sha256: Text,
    },
    Image {
        path: Text,
        sha256: Text,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootFile {
    pub path: Text,
    pub sha256: Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootSigner {
    pub subject: Text,
    pub fingerprint: Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteOrder {
    pub source: Source,
    pub disk: Text,
    pub device: Text,
    pub serial: Text,
    pub size: u64,
    pub mode: Mode,
    pub clear_first: bool,
    pub boot_file: Option<BootFile>,
    pub name: Option<Text>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemAsk {
    /// Its progress comes as `SystemFact::WriteProgress`.
    WriteStick(WriteOrder),
    Install,
    Uninstall,
    RemoveBootEntry,
    RestartIntoFirmware,
    RestartFromStick {
        device: Text,
    },
    ReadBitLocker,
    BitLocker(BitLockerStep),
    /// Answered with recovery keys as `SecretText`.
    ReadRecoveryKey,
    ReadVersion {
        disk: Text,
        serial: Text,
        size: u64,
        source: Option<Source>,
    },
    ListBootFiles,
    JudgeBoot {
        disk: Text,
        serial: Text,
        size: u64,
        sha256: Text,
        want: Text,
        source: Source,
    },
    CheckBootFile {
        path: Text,
        source: Source,
    },
    FindImages {
        dropped: Vec<Text>,
    },
    CheckAgain,
    /// The desktop adapter makes the Demo's sticks folder, `BOOTSTICK` and `STICK-2`.
    PrepareDemoSticks,
}

pub enum SystemAnswer {
    WriteStick(Result<(), Reason>),
    Install(Result<Text, Reason>),
    Uninstall(Result<(), Reason>),
    RemoveBootEntry(Result<(), Reason>),
    RestartIntoFirmware(Result<(), Reason>),
    RestartFromStick(Result<(), Reason>),
    ReadBitLocker(Option<BitLocker>),
    BitLocker {
        step: BitLockerStep,
        result: Result<(), Reason>,
    },
    RecoveryKey(Result<Vec<RecoveryKey>, Reason>),
    VersionRead {
        disk: Text,
        serial: Text,
        size: u64,
        result: Result<(Option<Text>, Option<Text>), Reason>,
        signed: Option<bool>,
    },
    BootFiles {
        folder: Text,
        files: Vec<Text>,
    },
    BootJudged {
        disk: Text,
        serial: Text,
        size: u64,
        sha256: Text,
        want: Text,
        signed: Option<bool>,
    },
    BootFileChecked {
        path: Text,
        result: Result<(BootFile, BootSigner), Reason>,
    },
    ImagesFound(Vec<ImageFinding>),
    CheckedAgain,
    PrepareDemoSticks(Result<(), Reason>),
}

// One own type per ask, each a `Request` (02 §2.2).

pub struct WriteStick(pub WriteOrder);
request!(WriteStick => Result<(), Reason>);

pub struct Install;
request!(Install => Result<Text, Reason>);

pub struct Uninstall;
request!(Uninstall => Result<(), Reason>);

pub struct RemoveBootEntry;
request!(RemoveBootEntry => Result<(), Reason>);

pub struct RestartIntoFirmware;
request!(RestartIntoFirmware => Result<(), Reason>);

pub struct RestartFromStick {
    pub device: Text,
}
request!(RestartFromStick => Result<(), Reason>);

pub struct ReadBitLocker;
request!(ReadBitLocker => Option<BitLocker>);

pub struct BitLockerDo(pub BitLockerStep);
request!(BitLockerDo => Result<(), Reason>);

pub struct ReadRecoveryKey;
request!(ReadRecoveryKey => Result<Vec<RecoveryKey>, Reason>);

pub struct ReadVersion {
    pub disk: Text,
    pub serial: Text,
    pub size: u64,
    pub source: Option<Source>,
}
request!(ReadVersion => Result<(Option<Text>, Option<Text>), Reason>);

pub struct ListBootFiles;
request!(ListBootFiles => (Text, Vec<Text>));

pub struct JudgeBoot {
    pub disk: Text,
    pub serial: Text,
    pub size: u64,
    pub sha256: Text,
    pub want: Text,
    pub source: Source,
}
request!(JudgeBoot => Option<bool>);

pub struct CheckBootFile {
    pub path: Text,
    pub source: Source,
}
request!(CheckBootFile => Result<(BootFile, BootSigner), Reason>);

pub struct FindImages {
    pub dropped: Vec<Text>,
}
request!(FindImages => Vec<ImageFinding>);

pub struct CheckAgain;
request!(CheckAgain => ());

pub struct PrepareDemoSticks;
request!(PrepareDemoSticks => Result<(), Reason>);
