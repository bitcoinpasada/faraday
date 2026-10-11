//! Faraday's app: the screens, the Wallets tab's session, the Inbox and the
//! Outbox, and the stick rule (`PLAN.md` §5).
//!
//! It is an [`osk_shell_api::App`], so the shells drive it with events and
//! read its frame, as they drive OpenSigner. Removable storage is the one
//! thing upstream's interface has no event for, so this crate adds a
//! second channel beside it: the shell reports the sticks it sees and the
//! files read from them through [`Faraday::storage`], and takes the
//! reads and writes this app asks for from
//! [`Faraday::poll_storage`]. The app itself does no I/O.
//!
//! Keys are typed, scanned or loaded from a vault for the session only;
//! PSBTs and wallets move by stick or QR. Test wallets come in on a test
//! stick (`testkit`, built only with the `testkit` feature).

pub mod backup;
pub mod backups;
pub mod bip85;
pub mod boot_import;
pub mod catalog;
pub mod chart_edit;
pub mod create;
pub mod decode;
pub mod explore;
pub mod family;
pub mod flow;
pub mod forms;
mod fresh;
pub mod glance;
pub mod glance_sheet;
pub mod gpg;
pub mod inbox;
pub mod inputs;
pub mod keygen;
pub mod learn;
pub mod lightning;
pub mod medium;
pub mod memory;
pub mod paper;
pub mod pdf;
pub mod picture;
pub mod plan;
pub mod restore;
pub mod secret_text;
pub mod secrets;
pub mod secureboot;
pub mod seeds;
pub mod silent;
pub mod stick_settings;
#[cfg(feature = "testkit")]
pub mod testkit;
pub mod tools;
pub mod transfer;
pub mod ui;
pub mod upgrade;
pub mod vanity;
pub mod vaults;
pub mod wallet;
pub mod wordlist;

mod backups_screen;
mod bip85_screen;
mod boot_import_screen;
mod compact;
mod edit;
mod held;
pub use compact::OskPress;
pub use medium::Medium;
mod compact_screens;
mod explore_screen;
mod family_screen;
mod family_text;
mod guide;
mod keygen_screen;
mod lightning_screen;
mod motion;
mod screens;
mod seeds_screen;
mod silent_screen;
mod tools_screen;
mod transfer_screen;
mod upgrade_screen;
mod vanity_screen;
mod vault_screens;
mod wordlist_screen;

use std::collections::{BTreeSet, VecDeque};

use osk_shell_api::{App, Command, DisplayInfo, Event, Frame, Key as KeyIn, TouchPhase};
use osk_ui::{Canvas, Rect};

use wallet::{FileKind, Session, Spend, fp_text};

/// The version shown on Settings.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The build ID the build scripts set (`docs/DECISIONS.md` F5): the
/// kernel release string's part after `-faraday-`. `None` for a plain
/// `cargo build`, which sets no such environment variable.
pub const BUILD: Option<&str> = option_env!("FARADAY_BUILD");

/// What About and Settings show for the running build: `BUILD` described
/// (`docs/DECISIONS.md` F5) when set, else the crate's version, marked as
/// a local build.
pub fn version_label() -> String {
    match BUILD {
        Some(build) => crate::upgrade::describe(build),
        None => format!("{VERSION} (local build)"),
    }
}

/// Minutes without input before the session locks, at first.
pub const IDLE_LOCK_MIN: u16 = 10;
/// Minutes without input before the machine powers off, at first,
/// counted from the last input across the lock.
pub const IDLE_OFF_MIN: u16 = 20;
/// How long before an idle lock its warning comes up, at most: half the
/// lock time when that is shorter.
pub const IDLE_WARN_MS: u64 = 5 * 60_000;

/// The exit code that asks the app loop for a fresh process
/// (`PLAN.md` §5.3).
pub const RESTART_CODE: u8 = 75;

/// A stick or card the shell sees, with the files on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickInfo {
    /// Where the shell finds it; the app hands this back unchanged.
    pub id: String,
    /// The name a person knows it by: its volume label.
    pub label: String,
    /// Whether it is the medium the machine booted from.
    pub boot: bool,
    /// The files at its top level, with their sizes.
    pub files: Vec<(String, u64)>,
}

/// A boot partition the boot copier has, while the app upgrades a stick
/// (`PLAN.md` §5.5).
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
    /// A picture on a stick: the QR codes the disk process read from it,
    /// and its bytes as they are. The app keeps and writes the bytes and
    /// never decodes them (`PLAN.md` §12 item 6).
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
    /// theirs (`PLAN.md` §4.6).
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
    /// Write a file to a stick: a new file, read back and compared, never
    /// over a file already there.
    Write {
        /// The stick's id.
        stick: String,
        /// The name to write under.
        name: String,
        /// The contents.
        bytes: Vec<u8>,
    },
    /// Read the QR codes in a PNG on a stick. The shell decodes the image;
    /// the app sees only what the codes hold.
    ReadQr {
        /// The stick's id.
        stick: String,
        /// The image.
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
    /// Have the boot copier read the boot partition that holds the
    /// running Faraday (`PLAN.md` §5.5).
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
    /// hold nothing secret (`PLAN.md` §5.2).
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

/// The screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Home.
    Home,
    /// The Wallets tab's first page: every flow, by what the person has.
    Start,
    /// The Wallets tab: the wallets, the wallet card, and every flow
    /// (`docs/WALLETS.md`).
    Wallets,
    /// Signing a transaction.
    Spend,
    /// Inbox and Outbox.
    Files,
    /// A stick visit.
    Visit,
    /// Typing a seed.
    Entry,
    /// Backing up a wallet.
    Backup,
    /// Signing a message.
    Message,
    /// Checking a signed message.
    CheckMessage,
    /// Creating a wallet.
    Create,
    /// Restoring a wallet.
    Restore,
    /// Settings and About.
    Settings,
    /// The vaults the Inbox and Outbox hold.
    Vaults,
    /// Every wallet known and where its backup is (`docs/SIMPLIFY.md`
    /// §5.1).
    Backups,
    /// Making a vault.
    CreateVault,
    /// Making a new key.
    KeyGen,
    /// BIP-85: child seeds, keys and passwords from a loaded key.
    Bip85,
    /// Silent payments: a loaded key's address and scan key.
    Silent,
    /// A loaded key's public side at any path.
    Explore,
    /// A Lightning node's key.
    Lightning,
    /// OpenSigner's calculators.
    Tools,
    /// Unlocking one slot of a vault.
    Unlock,
    /// An open vault's contents.
    VaultContents,
    /// The Spend tab (`docs/FAMILY.md`).
    Family,
    /// A vanity address for a loaded key.
    Vanity,
    /// A transaction taken apart.
    Decode,
    /// Tools: every flow on one page, with its standards.
    Catalog,
    /// The online app as the device's QR link: files out as codes, in by
    /// camera ([`transfer`]). Only when [`Faraday::online`].
    Transfer,
    /// Upgrading another Faraday stick from the one Faraday started from
    /// ([`upgrade`]). Never in the online app.
    Upgrade,
}

/// A screen and the sheet over it, if any.
type ScreenKey = (Screen, Option<Sheet>);
/// A scrolled region: the screen and sheet, and which of their regions.
type RegionKey = (ScreenKey, ui::Slot);

/// The step column's open card as last drawn, and the change under way
/// (`docs/MOTION.md` §3.5).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Disclosed {
    /// The screen and sheet the column was drawn on.
    key: ScreenKey,
    /// The card open.
    open: Option<usize>,
    /// Its body's height, units.
    body_h: f32,
    /// The card closing, and its body's height.
    closing: Option<(usize, f32)>,
    /// The tick the change started at (`None` until one comes).
    at: Option<u64>,
    /// The change is under way.
    moving: bool,
}

/// Everything a press can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Go to a screen.
    Nav(Screen),
    /// Open Backups on loaded wallet `i`.
    BackupsOf(usize),
    /// Open Back up a wallet on loaded wallet `i` at its checklist, from
    /// the plan its chart shows (`docs/NEW-WALLET.md` §6.2). Nothing is
    /// made For the stick by opening it: each item offers what it makes.
    BackupChecklist(usize),
    /// Finish the backup first (`docs/NEW-WALLET.md` §14.3): wallet `i`'s
    /// checklist, or its plan when none is made.
    BackupFirst(usize),
    /// Open, or close, the wallet's chart as its own page on a small
    /// panel.
    Glance(bool),
    /// A press on the wallet's chart or one of its sheets
    /// (`docs/NEW-WALLET.md` §9).
    Chart(glance_sheet::ChartAction),
    /// Show the Backups screen's wallet `i`, a page each on a small panel.
    BackupsPage(usize),
    /// A vault screen's action.
    Vault(vaults::VaultAction),
    /// Open seed entry, for a slot's fingerprint or for any key.
    Entry(Option<[u8; 4]>),
    /// Add the typed key.
    EntryAdd,
    /// Type the words from another list, by its place in
    /// `forms::LANGUAGES`.
    EntryLanguage(u8),
    /// A key of the on-screen keyboard a list's words are typed on.
    EntryKey(char),
    /// That keyboard's backspace.
    EntryKeyBack,
    /// That keyboard's shift (Korean's doubled consonants).
    EntryShift,
    /// Take the word offered at this place among the candidates.
    EntryCandidate(u8),
    /// Show the lists other than English, or hide them.
    EntryLanguages,
    /// Type another form of key, by its place in `forms::Form::ALL`.
    EntryForm(u8),
    /// Shows the SLIP-39, codex32 and Seed XOR chips.
    EntryOtherForms,
    /// The typed share, string or part, collected.
    EntryPart,
    /// The collected parts put together as a key.
    EntryRecover,
    /// Clear the typed words.
    EntryClear,
    /// Choose a wallet on the Wallets tab.
    PickWallet(usize),
    /// Choose a key in no wallet on the Wallets tab, by fingerprint.
    PickKey([u8; 4]),
    /// Remove a wallet from the session.
    RemoveWallet(usize),
    /// Remove every key from the session.
    RemoveKeys,
    /// Start signing the Inbox PSBT at this index.
    StartSpend(usize),
    /// Open or close a step card.
    Step(u8),
    /// Close a step card as done and open the next.
    StepNext(u8),
    /// Show or hide the full transaction table.
    ToggleTable,
    /// Sign with every session key the transaction names.
    SignHere,
    /// Add the signatures in this Inbox file.
    Collect(usize),
    /// Make the signed PSBT For the stick again, after a Remove. Finish
    /// makes it the moment the transaction is finished.
    SignedToOutbox,
    /// Make the finished transaction For the stick again, after a Remove.
    TxToOutbox,
    /// The summary panel's action.
    Primary,
    /// Load the wallet in this Inbox file.
    LoadWallet(usize),
    /// Remove an Inbox file.
    InboxRemove(usize),
    /// Remove an Outbox file.
    OutboxRemove(usize),
    /// Show this stick on the visit screen.
    VisitStick(usize),
    /// Choose an Outbox file to write.
    VisitOut(usize),
    /// Tick or untick the settings file on a stick visit.
    VisitSettings,
    /// Choose a stick file to copy.
    VisitIn(usize),
    /// Unselect all: none of the stick's files comes in.
    VisitInAll,
    /// Choose an Inbox file to write to the stick, by its index in the
    /// Inbox: one that may be a secret opens the secret sheet first.
    VisitInbox(usize),
    /// Choose the settings and every Outbox file but an unprotected
    /// secret, or none when all of those are chosen.
    VisitOutAll,
    /// A stick visit column's scrollbar: held and dragged.
    VisitBar(Column),
    /// Write the chosen Outbox files.
    VisitWrite,
    /// Copy the chosen stick files.
    VisitCopy,
    /// Copy the chosen stick files, and load as keys any that are once
    /// the stick is pulled.
    VisitCopyAndLoad,
    /// Load the key an Inbox file's words spell.
    LoadKey(usize),
    /// Scan a SeedQR into Add a key.
    ScanSeed,
    /// Scan one part of a key into Add a key's other forms: a SLIP-39
    /// share, a codex32 string, or a Seed XOR part's SeedQR or words.
    ScanPart,
    /// Choose or drop one of the other shares that sign a threshold
    /// spend with this one.
    TOther(u32),
    /// The carry file to the Outbox, for the share that signs next.
    CarryToOutbox,
    /// Type the BIP-39 passphrase in Add a key, or go back to the words.
    EntryPassphrase,
    /// Seal the signed-amount memory into an open vault on lock, or not.
    SealAmounts(bool),
    /// Lock: wipe the session and start a fresh process.
    Lock,
    /// Lock, first listing what is not saved in a vault when one is open.
    LockAsk,
    /// Leave the stick untouched and keep working.
    NotNow,
    /// Ask to power off.
    PowerAsk,
    /// Power off.
    PowerOff,
    /// Minutes without input before locking; 0 for never.
    IdleLock(u16),
    /// Minutes without input before powering off; 0 for never.
    IdleOff(u16),
    /// Believe a new pointer, said with input already believed.
    InputUse(u32),
    /// Keep a new device out until it is unplugged.
    InputIgnore(u32),
    /// One of wallet n's public files to the Outbox, as `public_out`
    /// numbers them: what the backup's public files item offers.
    PublicOut(usize, u8),
    /// Remove from For the stick the files [`Action::PublicOut`] makes
    /// for the same wallet and number.
    PublicRemove(usize, u8),
    /// The account key in wallet n's slot k, held here, to the Outbox
    /// for the cosigners.
    WalletKeyOut(usize, u8),
    /// That key as a BIP 129 key record, signed, to the Outbox.
    WalletKeyBsms(usize, u8),
    /// Open the stick visit from a flow, which it returns to when the
    /// stick is pulled.
    VisitFrom(Screen),
    /// Restore the wallet in record r of open vault v, with the vault's
    /// keys for its slots.
    RFromVault(usize, usize),
    /// Move the session to this network.
    Network(osk_bip::keys::Network),
    /// The online app's not-air-gapped warning acknowledged: mainnet,
    /// if it was asked for, follows.
    AirgapUnderstood,
    /// Close a sheet.
    Cancel,
    /// Set the display scale, in percent.
    Scale(u16),
    /// Draw in the dark or the light palette.
    Theme(ui::Theme),
    /// Reduce motion on or off.
    ReduceMotion(bool),
    /// Guided mode on or off.
    Guided(bool),
    /// On a small panel, a step's walk-through, or a page's, shown or
    /// put away: the step's place, or [`ABOUT_PAGE`] for the page.
    About(u8),
    /// Show the spend's signed PSBT as a QR code.
    QrSigned,
    /// Show the spend's PSBT as it stands, for a cosigner.
    QrPart,
    /// Show a wallet's descriptor as a QR code.
    QrWallet(usize),
    /// Show an Outbox file as a QR code.
    QrOutbox(usize),
    /// Transfer: send file n of the Downloads list as codes.
    TransferSend(usize),
    /// Transfer: receive by camera into Downloads.
    TransferReceive,
    /// Transfer: open Downloads in the file manager.
    TransferOpenFolder,
    /// Settings → Upgrade a Faraday stick.
    UpgradeOpen,
    /// The upgrade: the n-th stick in to upgrade, when more than one is.
    UpgradePick(u8),
    /// The upgrade: write the stick on show.
    UpgradeWrite,
    /// The upgrade: another stick, from the same source.
    UpgradeAgain,
    /// The QR view as BBQr (`true`) or UR.
    QrFormat(u8),
    /// The QR view's bytes of data a frame.
    QrPartSize(usize),
    /// Milliseconds each frame of an animated code stays up.
    QrSpeed(u64),
    /// The QR sheet's code as a labelled picture, to the Outbox.
    QrPng,
    /// A public file a flow makes, shown as a QR code.
    ShowCode(Code),
    /// A public file a flow makes, as a labelled picture of its code to
    /// the Outbox: what the QR sheet's PNG makes, without the sheet.
    CodePng(Code),
    /// Open a wallet's card.
    OpenWallet(usize),
    /// Start backing up a wallet.
    Backup(usize),
    /// Open or close the list of loaded wallets under the backup's
    /// wallet chip.
    BWallets,
    /// Open or close an item of the backup's checklist, by [`bstep`].
    BStep(u8),
    /// Open the checklist item after this one; on the envelopes, close
    /// them as done.
    BNext(u8),
    /// Open or close a question of the backup's plan, by [`qstep`].
    BQ(u8),
    /// Open the plan's question after this one.
    BQNext(u8),
    /// Fill the plan's answers from a preset, by its place in
    /// [`plan::Preset::ALL`].
    BPreset(u8),
    /// Tick or untick a row of one of the plan's questions: the question
    /// by [`qrow`], then the row.
    BAnswer(u8, u8),
    /// Start or stop typing place n's name (only with a vault open).
    BName(u8),
    /// The plan made: on to the checklist, the plan saved into the open
    /// vault.
    BChecklist,
    /// Back from the checklist to the plan.
    BPlan,
    /// Show the seed of this session key.
    BKey(usize),
    /// Show or hide the words.
    BReveal,
    /// Standard or compact SeedQR.
    BCompact(bool),
    /// Pin this grid row.
    BPin(usize),
    /// Start or stop typing the digits back.
    BCheck,
    /// Scan the copy of the seed on screen.
    BScan,
    /// Clear the typed digits.
    BCheckClear,
    /// Vault `v`'s item: save into the open vault, in one press,
    /// everything the plan puts in that vault: its seeds, each passphrase
    /// the plan keeps there, and the wallet where the plan puts it there.
    BVaultSave(u8),
    /// Make the checklist's public files again: the template, the sheet
    /// or the shares, the files for the software.
    BFilesMake,
    /// A vault's item, the open vault holding another vault's seed: lock
    /// that vault and make a new one, back to the item.
    BNewVault,
    /// The seed shown as a file: the secret sheet for it.
    BFile,
    /// The template's word count.
    BWords(usize),
    /// Keys left off each split sheet.
    BOmit(usize),
    /// Start signing a message.
    SignMessage,
    /// Open or close a message card.
    MStep(u8),
    /// Close a message card as done and open the next.
    MNext(u8),
    /// The wallet to sign with.
    MWallet(usize),
    /// Move the address index by this much.
    MIndex(i32),
    /// Start or stop typing the message.
    MType,
    /// BIP-322 (true) or BIP-137.
    MFormat(bool),
    /// Sign the message.
    MSign,
    /// Put the signed message in the Outbox.
    MOut,
    /// Show the signed message as a QR code.
    MQr,
    /// Check the signed message in this Inbox file.
    CheckMessage(usize),
    /// Start creating a wallet.
    CreateWallet,
    /// Open or close a create card.
    CStep(u8),
    /// Close a create card as done and open the next.
    CNext(u8),
    /// The kind, by index into `NewKind::ALL`.
    CKind(u8),
    /// Expands the Kind card's other eight kinds.
    CMoreKinds,
    /// Change the signatures needed.
    CM(i8),
    /// Change the number of keys.
    CN(i8),
    /// Fill a slot with a key loaded here.
    CSlotHere(u8, [u8; 4]),
    /// The secret on its way out into the open vault.
    SecretVault,
    /// The person has read what the secret gives away, or unread it.
    SecretAck,
    /// The secret to the Outbox, unprotected.
    SecretUnprotected,
    /// The form the secret's file takes, by index.
    SecretForm(u8),
    /// Open BIP-85.
    Bip85,
    /// Open silent payments.
    Silent,
    /// Open Explore.
    Explore,
    /// Open Explore on this loaded seed, by fingerprint.
    ExploreKey([u8; 4]),
    /// Open the Lightning node key.
    Lightning,
    /// Open Tools.
    Tools,
    /// A calculator, by its place in `Tool::ALL`.
    TTool(u8),
    /// The calculator's mode row.
    TMode(u8),
    /// Empty the calculator's field.
    TClear,
    /// The node key from this loaded key, as ldk-node reads it.
    LKey([u8; 4]),
    /// The node key from an aezeed typed here.
    LAezeed,
    /// Type into the aezeed's passphrase, or back into its words.
    LPassphrase,
    /// Read the aezeed.
    LResolve,
    /// Show or hide the private key.
    LShow,
    /// The private key into the open vault.
    LVault,
    /// The private key out unprotected, through the secret sheet.
    LOut,
    /// Explore this key, by fingerprint.
    XKey([u8; 4]),
    /// A path preset, by its place in `explore::PRESETS`.
    XPreset(u8),
    /// The script for a path that names none.
    XScript(u8),
    /// The path's last index up or down.
    XIndex(i32),
    /// Open or close a silent payments card.
    SStep(u8),
    /// The key, by fingerprint.
    SKey([u8; 4]),
    /// The label up or down.
    SLabel(i8),
    /// The address as a QR code: as it is, or as a `bitcoin:` link.
    SQr(bool),
    /// The public record to the Outbox.
    SRecord,
    /// The scan key into the open vault.
    SScanVault,
    /// The scan key out unprotected, through the secret sheet.
    SScanOut,
    /// Done with this silent payments card.
    SNext,
    /// Add the key's silent payments wallet to the session's wallets.
    SAddWallet,
    /// Open session wallet n, a silent payments wallet, on its page.
    SWallet(usize),
    /// Check whether Inbox file n, a transaction, pays the wallet.
    SCheck(usize),
    /// Open or close a BIP-85 card.
    PStep(u8),
    /// The key BIP-85 derives from, by fingerprint.
    PKey([u8; 4]),
    /// The application, by its place in `bip85::APPS`.
    PApp(u8),
    /// The length up or down a step.
    PLength(i8),
    /// The index up or down.
    PIndex(i32),
    /// Done with this BIP-85 card.
    PNext,
    /// Show or hide the value.
    PShow,
    /// The value into the open vault.
    PVault,
    /// Child words as a key of this session.
    PLoad,
    /// The value out unprotected, through the secret sheet.
    POut,
    /// Split the key's words into this many Seed XOR parts.
    BXor(u8),
    /// codex32 shares of the key's seed, `k` of `n`.
    BCodex32(u8, u8),
    /// Put the other paper form away.
    BPaperHide,
    /// Read with this camera, by its place in the shell's list.
    ScanCamera(u8),
    /// Open the Learn pages for the screen on show.
    Learn,
    /// The docked keyboard of a small panel.
    Osk(compact::OskPress),
    /// Show the Learn sheet's page.
    LearnPage(u8),
    /// The Learn sheet's first row, **Spending, step by step**: closes
    /// the sheet and opens the Spend tab (`docs/SIMPLIFY.md` §6.2).
    LearnSpend,
    /// **What is a share?** beside the backup plan's Its own share: the
    /// Learn sheet on the page about shares (`docs/NEW-WALLET.md` §4.2).
    LearnShares,
    /// The word-list sheet, or a link to it.
    WordList(wordlist::WordListAction),
    /// Make a new key, for this Create slot or for the session.
    KeyGen(Option<u8>),
    /// Open or close a new-key card.
    KStep(u8),
    /// 12 or 24 words.
    KWords(u8),
    /// Choose an option of the Randomness card, by its place in
    /// `keygen::WAYS`.
    KWay(u8),
    /// Open or close a group of the Randomness card: 0 the options
    /// verifiable by hand, 1 the ones the device computes.
    KGroup(u8),
    /// Take a source into a mix or out of it, by its place in
    /// `keygen::Source::MIXABLE`.
    KMix(u8),
    /// A die's face.
    KRoll(u8),
    /// A coin: heads or tails.
    KFlip(bool),
    /// A die's face, read as a coin flip: 1 to 3 tails, 4 to 6 heads.
    KDie(u8),
    /// Coin flips entered as a die's faces (true) or as a coin's sides.
    KByDie(bool),
    /// Entries typed as one string into a box (true), or pressed one at
    /// a time.
    KTyping(bool),
    /// A card's rank.
    KRank(u8),
    /// A card's suit.
    KSuit(u8),
    /// A hex digit's value.
    KHex(u8),
    /// Take the camera's picture.
    KFrame,
    /// Take back the last entry.
    KUndo,
    /// Forget this source's entries.
    KClear,
    /// Done with this card.
    KNext,
    /// Run the self-test again (Settings › About).
    SelfTestRun,
    /// Show or hide the words.
    KShow,
    /// The Quiz's last button: loads SLIP-39 shares' key, or leaves the
    /// flow once a BIP-39 key is locked in.
    KAdd,
    /// Lock in, on New key's Key card: adds the key.
    KLock,
    /// Typing goes to the Key card's passphrase field: 0 the first, 1
    /// the second.
    KPassField(u8),
    /// The quiz's candidate in this slot.
    KQuiz(u8),
    /// Ask the word that was answered wrongly again.
    KQuizRetry,
    /// Skip the quiz: the first press asks, the second skips.
    KSkip,
    /// Throw the entries away and start again.
    KAgain,
    /// New key as SLIP-39 shares (true) or BIP-39 words (false).
    KForm(bool),
    /// A SLIP-39 share's word count, chosen without leaving the card.
    KSlipWords(u8),
    /// Shares needed, one more or one fewer.
    KSlipM(i8),
    /// Shares dealt, one more or one fewer.
    KSlipN(i8),
    /// The next or the previous share on screen.
    KShare(i8),
    /// Open New key making SLIP-39 shares, for the session.
    KeyGenSlip39,
    /// A Spend tab action.
    Family(family::FamilyAction),
    /// A vanity screen action.
    Vanity(vanity::VanityAction),
    /// Fill a slot with the account key in an Inbox file.
    CSlotFile(u8, usize),
    /// Empty a slot.
    CSlotClear(u8),
    /// Open or close the passphrase fields under a slot whose key was
    /// loaded from BIP-39 words without one.
    CPassOpen(u8),
    /// Typing goes to that passphrase field: 0 the first, 1 the second.
    CPassField(u8),
    /// Lock in the slot's key with the passphrase typed.
    CPassLock,
    /// Leave a slot for a cosigner's key, to add later.
    CSlotLater(u8),
    /// This slot's account key to the Outbox, for the cosigners.
    CKeyOut(u8),
    /// This slot's account key as a QR code.
    CKeyQr(u8),
    /// This slot's key as a BIP 129 key record, signed, to the Outbox.
    CKeyBsms(u8),
    /// Throw away the creation under way and start again.
    CreateOver,
    /// Create's Back up card: Back up a wallet on the wallet just made,
    /// with the plan's preset `k` applied.
    CBackup(u8),
    /// Start restoring a wallet.
    RestoreWallet,
    /// Open or close a restore card.
    RStep(u8),
    /// Close a restore card as done and open the next.
    RNext(u8),
    /// Restore the wallet in this Inbox file.
    RUse(usize),
    /// Take or leave the share in this Inbox file.
    RShare(usize),
    /// Rebuild the wallet from the shares chosen.
    RRebuild,
    /// Description's **I do not have it**: the wallet is made from the
    /// seeds and the cosigners' xpubs at the kind's standard path.
    RSeeds,
    /// Restore's Kind, by its place in `NewKind::ALL`
    /// (`docs/NEW-WALLET.md` §12.1).
    RKind(u8),
    /// Restore's Kind card shows every kind.
    RMoreKinds,
    /// The shortcut's **From Files**: the wallet description in Files.
    RFromFiles,
    /// A key of the description whose seed is not here, or here after
    /// all, by its place in the wallet.
    RAbsent(u8),
    /// Typing goes to the open slot's passphrase field: 0 the first, 1
    /// the second.
    RPassField(u8),
    /// The open slot's **Type the words**, with its passphrase.
    RSlotWords,
    /// The open slot's **Scan a SeedQR**, with its passphrase.
    RSlotScan,
    /// A wallet from a loaded key in no wallet, by fingerprint: Restore's
    /// Seeds card with that key in its first slot, with this many keys
    /// (1 for a wallet of that key alone, 2 to add another).
    KeyWallet([u8; 4], u8),
    /// A wallet made from the seeds in hand (`seeds.rs`).
    Seeds(seeds::SeedsAction),
    /// A slider pressed, dragged or stepped: which slider, the value.
    Slide(u8, u8),
    /// Rebuild the wallet from every share in the Inbox, and open it.
    RestoreShares,
    /// Read QR codes with the camera into Files.
    Scan,
    /// Type a new name for the chosen wallet.
    Rename,
    /// Make a PDF of an Inbox sheet (online only).
    PdfInbox(usize),
    /// Make a PDF of an Outbox sheet (online only).
    PdfOutbox(usize),
    /// Make a backup file For the stick again: 0 template, 1 descriptor,
    /// 2 multisig config, 3 backup sheet, 4 split shares, 5 wallet .json.
    /// Making the checklist makes the ones it calls for.
    BOut(u8),
    /// Decode the transaction this spend finished.
    DecodeFinished,
    /// Open the Tools page's tile n.
    Catalog(u8),
    /// Load the wallets the Inbox describes, with the seeds here.
    InboxLoad,
    /// Leave out, or take back, the Inbox's wallet n in its next load.
    InboxChoose(usize),
    /// Make the seed with this fingerprint a wallet.
    PotentialOpen([u8; 4]),
    /// Open the OpenSigner backup in this Inbox file with its passphrase.
    BackupOpen(usize),
    /// Make the Inbox's account xpub n a watch-only wallet.
    XpubOpen(usize),
    /// The potential wallet's kind, by its place in `inbox::KINDS`.
    PotentialKind(u8),
    /// Show or mask the potential wallet's passphrase.
    PotentialShow,
    /// Typing goes to the potential wallet's passphrase.
    PotentialType,
    /// Make the potential wallet, or open the backup.
    PotentialMake,
    /// Decode the transaction in this Inbox file.
    DecodeInbox(usize),
    /// Show or hide the decoded transaction's hex.
    DecodeHex,
    /// The decoded transaction as a QR code.
    QrDecoded,
    /// Show or hide the finished transaction's hex on the spend.
    SpendHex,
    /// Write the Outbox to a stick: the visit now when nothing secret was
    /// held, else what a lock wipes and keeps first.
    WriteAsk,
    /// The boot import's sheet.
    Import(boot_import::ImportAction),
}

/// The largest file read from a stick, as the disk process reads them
/// (`faraday-files` `MAX_READ`): a larger one is listed and not read.
pub const READ_MAX: u64 = 18 * 1024 * 1024;

/// A file named as Faraday names a seed it writes out: its words, its
/// SeedQR, a BIP-85 child.
fn seed_by_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with("words.txt") || lower.ends_with("seedqr.png") || lower.starts_with("bip85-")
}

/// What a stick visit calls a file it copies in, by its extension: any
/// file but the settings file is copied, and one whose extension says
/// nothing is a File, whatever its contents turn out to be. `None` for
/// the settings file.
pub fn stick_kind(name: &str) -> Option<&'static str> {
    // The settings file is read at boot, not copied in.
    if stick_settings::is_file(name) {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "psbt" => Some("PSBT"),
        "txt" | "json" | "descriptor" => Some("Text"),
        "txn" => Some("Transaction"),
        "faraday-sheet" => Some("Sheet for printing"),
        "ofv" => Some("Vault"),
        "efi" => Some("EFI image"),
        "osk" => Some("Threshold spend, part-signed"),
        "oskb" => Some("OpenSigner backup"),
        "png" => Some("QR codes in a PNG"),
        _ => Some("File"),
    }
}

/// The words' BIP-39 numbers a drawn SeedQR holds: a Standard SeedQR's
/// digits four to a word, or a Compact SeedQR's 16 or 32 bytes. The
/// digits are read without the checksum, so a copy with one word wrong
/// still reads and the word is named. Wiped when dropped.
fn copy_indices(bytes: &[u8]) -> Option<zeroize::Zeroizing<Vec<u16>>> {
    if !bytes.is_empty() && bytes.iter().all(u8::is_ascii_digit) {
        if !matches!(bytes.len(), 48 | 96) {
            return None;
        }
        let mut out = zeroize::Zeroizing::new(Vec::with_capacity(bytes.len() / 4));
        for chunk in bytes.chunks(4) {
            let v = chunk
                .iter()
                .fold(0u16, |v, &c| v * 10 + u16::from(c - b'0'));
            if v >= 2048 {
                return None;
            }
            out.push(v);
        }
        return Some(out);
    }
    let m = osk_codec::seedqr::from_entropy(bytes, osk_bip::bip39::Language::English).ok()?;
    Some(zeroize::Zeroizing::new(m.indices().to_vec()))
}

/// The words a SeedQR holds, standard (digits) or compact (entropy), in
/// English; `None` for any other code.
fn seedqr_words(payload: &[u8]) -> Option<zeroize::Zeroizing<String>> {
    let lang = osk_bip::bip39::Language::English;
    let digits = payload.iter().all(u8::is_ascii_digit);
    let m = if digits && matches!(payload.len(), 48 | 96) {
        osk_codec::seedqr::from_digits(payload, lang).ok()?
    } else if matches!(payload.len(), 16 | 32) && std::str::from_utf8(payload).is_err() {
        osk_codec::seedqr::from_entropy(payload, lang).ok()?
    } else {
        return None;
    };
    let words = lang.words();
    let mut out = crate::secret_text::room();
    for (k, &i) in m.indices().iter().enumerate() {
        if k > 0 {
            out.push(' ');
        }
        out.push_str(words[usize::from(i)]);
    }
    Some(out)
}

/// The seed a code holds: a SeedQR, a CompactSeedQR, or a seed's words in
/// English, as words; `None` for anything else.
fn seed_in_code(payload: &[u8]) -> Option<zeroize::Zeroizing<String>> {
    if let Some(words) = seedqr_words(payload) {
        return Some(words);
    }
    let text = zeroize::Zeroizing::new(String::from_utf8_lossy(payload).into_owned());
    let mut out = crate::secret_text::room();
    for (k, w) in text.split_whitespace().enumerate() {
        if k > 0 {
            out.push(' ');
        }
        out.push_str(w);
    }
    let words = out.split(' ').count();
    (matches!(words, 12 | 15 | 18 | 21 | 24)
        && osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, out.as_str()).is_ok())
    .then_some(out)
}

/// Said on the camera when what it read would load a key with a stick
/// attached.
fn stick_keys(m: Medium) -> String {
    format!("Keys load only with no {} attached", m.noun())
}

/// A transaction being decoded, and where the person came from.
pub struct DecodeState {
    /// What it decoded to.
    pub decoded: decode::Decoded,
    /// The screen its back link returns to.
    pub back: Screen,
    /// The hex is shown.
    pub show_hex: bool,
}

/// A file in the Inbox or the Outbox.
#[derive(Debug, Clone)]
pub struct Item {
    /// Its name.
    pub name: String,
    /// Its contents.
    pub bytes: Vec<u8>,
    /// What it holds.
    pub kind: FileKind,
    /// A secret the person let out unprotected, whatever its contents
    /// read as (`docs/FLOWS.md` decision 6).
    pub secret: bool,
    /// A picture copied in from a stick: what its QR codes read as, which
    /// the disk process decoded ([`FileKind::Other`] when none read). Its
    /// bytes are the picture's, kept to be written out as they came.
    pub picture: Option<FileKind>,
}

/// An Inbox or Outbox file may be a seed or a share written in the clear:
/// its bytes are wiped when it goes, whatever it holds.
impl Drop for Item {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.bytes);
    }
}

impl Item {
    fn new(name: &str, bytes: Vec<u8>) -> Item {
        let kind = wallet::classify(name, &bytes);
        Item {
            name: name.to_string(),
            bytes,
            kind,
            secret: false,
            picture: None,
        }
    }

    /// What it holds: a picture's codes, or the file itself.
    pub fn holds(&self) -> FileKind {
        self.picture.unwrap_or(self.kind)
    }

    /// Who may read it: a secret let out is one, whatever its kind; a
    /// picture is what its codes hold.
    pub fn exposure(&self) -> secrets::Exposure {
        if self.secret {
            secrets::Exposure::Secret
        } else {
            self.holds().exposure()
        }
    }

    /// What a stick visit asks the person to say before it writes this
    /// Inbox file, on the secret sheet: `None` for a public or sealed file,
    /// written as it is.
    pub fn copy_ack(&self) -> Option<secrets::Ack> {
        if self.secret {
            return Some(secrets::Ack::Any);
        }
        match self.holds() {
            FileKind::Words | FileKind::SeedPart => Some(secrets::Ack::Seed),
            FileKind::Carry | FileKind::Entries => Some(secrets::Ack::Any),
            FileKind::Text | FileKind::Other => Some(secrets::Ack::Unknown),
            _ => None,
        }
    }
}

/// A sheet over the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sheet {
    /// A stick arrived while a secret is held.
    Lock,
    /// Lock asked for while a vault is open: what is sealed and what is
    /// not saved.
    LockAsk,
    /// Power off with files in the Outbox.
    Power,
    /// A QR code, or an animated run of them.
    Qr,
    /// The camera, reading QR codes.
    Scan,
    /// The online app on mainnet: this computer is not air-gapped.
    NotAirgapped,
    /// Locked for idleness: what waits in the Outbox.
    Locked,
    /// A new input device, waiting to be believed or ignored.
    NewInput,
    /// OpenSigner's Learn pages for the screen on show.
    Learn,
    /// A word list, every word with its number, over the screen it was
    /// opened from.
    WordList,
    /// A secret on its way out: into a vault, or the Outbox after a
    /// warning.
    SecretOut,
    /// Writing the Outbox to a stick, asked for while a secret is held:
    /// the lock it takes first.
    WriteOut,
    /// The idle lock coming: a countdown, and what the lock does.
    IdleWarn,
    /// A seed from the Inbox becoming a wallet: a passphrase or none, and
    /// its kind; or a sealed backup's passphrase.
    Potential,
    /// What the boot stick brought: remove the stick, unlock its vaults,
    /// choose what to import.
    Import,
    /// Something pressed that loads a key, with a stick attached: it
    /// carries on once the stick is pulled ([`Faraday::pull`]).
    Pull,
    /// A node or a line of the wallet's chart: what it is, where, and its
    /// actions (`docs/NEW-WALLET.md` §9).
    Chart,
}

/// What a scan pass saw of a code ([`StorageEvent::QrSeen`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    /// The frame's size.
    pub width: u16,
    /// The frame's height.
    pub height: u16,
    /// The code's corners in the frame.
    pub corners: [(u16, u16); 4],
    /// It read.
    pub read: bool,
}

/// Below this, in tenths of a pixel a module, a code is too fine for the
/// camera to read.
pub const TOO_FINE_TENTHS: u16 = 25;

/// Passes in a row that see a code too fine to read before the sheet
/// says so.
const TOO_FINE_PASSES: u32 = 3;

/// What the camera is reading codes for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScanPurpose {
    /// A file for Files: a PSBT, a descriptor, an xpub, a message.
    #[default]
    Files,
    /// A SeedQR for Add a key.
    Seed,
    /// One part of a key for Add a key's other forms.
    KeyPart,
    /// A TOTP setup code into a vault entry.
    VaultEntry,
    /// The hand-drawn copy of the seed with this fingerprint, on the
    /// backup's copy item: compared with it there, never loaded.
    CheckCopy(osk_bip::keys::Fingerprint),
    /// Transfer's Receive: anything, saved by the shell into Downloads,
    /// never into the Inbox.
    Transfer,
}

/// What the camera sheet holds.
#[derive(Default)]
pub struct ScanState {
    /// The newest preview frame: width, height, luma.
    pub frame: Option<(u16, u16, Vec<u8>)>,
    /// Its NV12 chroma plane, when the camera gives colour.
    pub chroma: Option<Vec<u8>>,
    /// What the scanner last saw of a code, and at which preview frame.
    pub seen: Option<(Seen, u32)>,
    /// Preview frames shown so far.
    pub frames: u32,
    /// Passes in a row that found a code too fine to read.
    pub too_fine: u32,
    /// A multi-part UR being put together.
    pub decoder: osk_codec::ur::Decoder,
    /// The last thing worth saying about what was read.
    pub note: Option<String>,
    /// Codes read so far.
    pub reads: usize,
    /// What the codes read are for.
    pub purpose: ScanPurpose,
    /// BBQr and numbered parts being put together.
    pub assembler: faraday_qr::Assembler,
}

impl ScanState {
    /// Whether a transfer in parts is part-way through.
    pub fn in_parts(&self) -> bool {
        self.assembler.busy() || self.decoder.progress().is_some()
    }
}

impl Drop for ScanState {
    fn drop(&mut self) {
        // A frame may hold a picture of a SeedQR.
        if let Some((_, _, luma)) = self.frame.as_mut() {
            zeroize::Zeroize::zeroize(luma);
        }
        if let Some(uv) = self.chroma.as_mut() {
            zeroize::Zeroize::zeroize(uv);
        }
    }
}

/// What a QR sheet shows.
pub struct QrView {
    /// What the code holds.
    pub title: String,
    /// One line under the title: the format, and the frame count.
    pub subtitle: String,
    /// The codes, cycled when there is more than one.
    pub frames: Vec<osk_codec::qr::QrMatrix>,
    /// The frame shown.
    pub frame: usize,
    /// When the next frame is due.
    next_ms: u64,
    /// What is shown, kept so the format and size can change.
    pub source: QrSource,
    /// UR or BBQr.
    pub format: QrFormat,
    /// Bytes of data a frame, when it is animated.
    pub part: usize,
    /// A secret: whoever scans it can use it.
    pub secret: bool,
    /// Public content worth keeping as a picture, as the caller says (a
    /// descriptor, a key, a signed message; not a PSBT or a transaction,
    /// which are handed over as files or in parts): while the view shows
    /// one code, the sheet offers it as a labelled PNG.
    pub public: bool,
    /// The lines under the title in that picture, one statement each.
    pub label: Vec<String>,
    /// The picture's file name.
    pub png_name: String,
    /// The text the code holds, shown whole under it: a wallet's
    /// descriptor, so it can be read against the code.
    pub whole: Option<String>,
}

/// A code on screen may be a seed: what it was made from is wiped when
/// the sheet closes. The codes themselves are `osk-codec`'s, which has no
/// way to wipe them.
impl Drop for QrView {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        match &mut self.source {
            QrSource::Psbt(b) => b.zeroize(),
            QrSource::Text(t) => t.zeroize(),
            QrSource::Key(k, a) => {
                k.zeroize();
                a.zeroize();
            }
            QrSource::File(n, b) => {
                n.zeroize();
                b.zeroize();
            }
        }
        self.title.zeroize();
        self.subtitle.zeroize();
    }
}

/// What a QR view shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrSource {
    /// A PSBT: `ur:crypto-psbt`, or BBQr `P`.
    Psbt(Vec<u8>),
    /// Text a wallet reads (a descriptor, a key): one code while it fits,
    /// else `ur:bytes`, or BBQr `U`.
    Text(String),
    /// A cosigner's account key: `ur:crypto-account` when it has one (a
    /// `wsh` multisig key, as SeedSigner shows its own), else as
    /// text; or BBQr `U` of the `[fingerprint/path]xpub` line.
    Key(String, Option<String>),
    /// Any other file, in the Faraday file envelope (`docs/QR.md`):
    /// `ur:bytes`, or BBQr `J`, as Faraday OS reads them.
    File(String, Vec<u8>),
}

/// A public file a flow makes that is offered as a code and as a
/// labelled picture of it, beside the file itself (`docs/QR.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// A wallet's descriptor, or a threshold wallet's record.
    Descriptor(usize),
    /// A wallet's multisig config, as Coldcard, Keystone and Passport
    /// scan it.
    MultisigConfig(usize),
    /// A wallet's BIP 129 descriptor record.
    Bsms(usize),
    /// The account key of Create's slot n, for the cosigners.
    Key(u8),
    /// That key's signed BIP 129 key record.
    KeyBsms(u8),
    /// The account key in wallet n's slot k, held here, as the wallet's
    /// descriptor writes it.
    WalletKey(usize, u8),
    /// That key's signed BIP 129 key record.
    WalletKeyBsms(usize, u8),
    /// The account key in wallet n's slot k, held elsewhere, as the
    /// wallet's descriptor writes it (`docs/NEW-WALLET.md` §9.3).
    CosignerKey(usize, u8),
    /// The silent payments address on show: as it is, or as a
    /// `bitcoin:` link.
    Silent(bool),
    /// The silent payments record, `silent-{fp}.txt`.
    SilentRecord,
    /// The signed message.
    Message,
    /// The selected GPG key's public certificate.
    GpgKey,
    /// A revocation certificate for the selected GPG key.
    GpgRevocation,
    /// A detached signature by the selected GPG key over Inbox file n.
    GpgSignature(usize),
}

/// How a QR view is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrFormat {
    /// Uniform Resources: what Sparrow and most wallets read.
    Ur,
    /// BBQr: what Coldcard writes and SeedSigner 0.8.7 and newer read.
    Bbqr,
    /// Specter's numbered text parts, `p1of3 …`: what SeedSigner reads a
    /// descriptor or a key in when it does not read BBQr. Text only.
    Parts,
}

impl QrFormat {
    /// The formats in the order the sheet lists them.
    pub const ALL: [QrFormat; 3] = [QrFormat::Ur, QrFormat::Bbqr, QrFormat::Parts];
}

/// The part sizes offered, in bytes of data a frame.
pub const QR_PARTS: [usize; 3] = [100, 160, 250];

/// The frame rates offered, as milliseconds a frame: 2, 3 and 5 a
/// second (`docs/QR.md`: 2 by default).
pub const QR_SPEEDS: [u64; 3] = [500, 333, 200];

type Frames = (String, Vec<osk_codec::qr::QrMatrix>);

impl QrView {
    /// A PSBT as `ur:crypto-psbt`: one code when it fits, else animated.
    pub fn psbt(title: &str, bytes: &[u8]) -> Result<QrView, String> {
        QrView::of(
            title,
            QrSource::Psbt(bytes.to_vec()),
            QrFormat::Ur,
            QR_PARTS[1],
        )
    }

    /// Text as one code while it fits version 25 at medium error
    /// correction (the rule for a descriptor), and as animated
    /// `ur:bytes` when it does not.
    pub fn text(title: &str, text: &str) -> Result<QrView, String> {
        QrView::of(
            title,
            QrSource::Text(text.to_string()),
            QrFormat::Ur,
            QR_PARTS[1],
        )
    }

    /// `source` written as `format`, `part` bytes a frame when animated.
    pub fn of(
        title: &str,
        source: QrSource,
        format: QrFormat,
        part: usize,
    ) -> Result<QrView, String> {
        use osk_codec::qr::{Ecc, Payload, encode_bounded};
        let (sub, frames) = match (&source, format) {
            (QrSource::Psbt(bytes), QrFormat::Ur) => {
                let single = osk_codec::ur::encode_psbt(bytes).to_ascii_uppercase();
                if let Ok(m) =
                    encode_bounded(Payload::Alphanumeric(single.as_bytes()), Ecc::Low, 20)
                {
                    ("ur:crypto-psbt · one code".to_string(), vec![m])
                } else {
                    let frames = animated(
                        osk_codec::ur::Encoder::psbt(bytes, part).map_err(|e| format!("{e:?}"))?,
                        bytes.len(),
                        part,
                    )?;
                    let sub = format!("ur:crypto-psbt · {} frames, shown in turn", frames.len());
                    (sub, frames)
                }
            }
            (QrSource::Psbt(bytes), QrFormat::Bbqr | QrFormat::Parts) => {
                bbqr_frames('P', bytes, part)?
            }
            (QrSource::Text(text), QrFormat::Parts) | (QrSource::Key(text, _), QrFormat::Parts) => {
                parts_frames(text, part)?
            }
            (QrSource::Text(text), QrFormat::Ur) => {
                if let Ok(m) = encode_bounded(Payload::Bytes(text.as_bytes()), Ecc::Medium, 25) {
                    ("Text · one code".to_string(), vec![m])
                } else {
                    ur_bytes_frames(text.as_bytes(), "ur:bytes", part)?
                }
            }
            (QrSource::Text(text), QrFormat::Bbqr) | (QrSource::Key(text, _), QrFormat::Bbqr) => {
                bbqr_frames('U', text.as_bytes(), part)?
            }
            (QrSource::Key(_, Some(ur)), QrFormat::Ur) => {
                let m = encode_bounded(
                    Payload::Alphanumeric(ur.to_ascii_uppercase().as_bytes()),
                    Ecc::Low,
                    20,
                )
                .map_err(|e| format!("{e:?}"))?;
                ("ur:crypto-account · one code".to_string(), vec![m])
            }
            (QrSource::Key(text, None), QrFormat::Ur) => {
                let m = encode_bounded(Payload::Bytes(text.as_bytes()), Ecc::Medium, 25)
                    .map_err(|e| format!("{e:?}"))?;
                ("Key · one code".to_string(), vec![m])
            }
            (QrSource::File(name, data), f) => {
                let env = faraday_qr::envelope::pack(&envelope_name(name), data, "file")
                    .map_err(str::to_string)?;
                match f {
                    QrFormat::Ur => ur_bytes_frames(&env, "File · ur:bytes", part)?,
                    QrFormat::Bbqr | QrFormat::Parts => bbqr_frames('J', &env, part)?,
                }
            }
        };
        let secret = match &source {
            QrSource::File(name, data) => {
                wallet::classify(name, data).exposure() == secrets::Exposure::Secret
            }
            _ => false,
        };
        Ok(QrView {
            title: title.to_string(),
            subtitle: sub,
            frames,
            frame: 0,
            next_ms: 0,
            source,
            format,
            part,
            secret,
            public: false,
            label: Vec::new(),
            png_name: String::new(),
            whole: None,
        })
    }

    /// The same source in another format or part size.
    pub fn redo(&self, format: QrFormat, part: usize) -> Result<QrView, String> {
        let secret = self.secret;
        QrView::of(&self.title, self.source.clone(), format, part).map(|mut v| {
            v.secret |= secret;
            v.public = self.public;
            v.label = self.label.clone();
            v.png_name = self.png_name.clone();
            v.whole = self.whole.clone();
            v
        })
    }

    /// The view as public content a picture may keep: the file `name`,
    /// the title and then `lines` under the code.
    pub fn public(mut self, name: &str, lines: Vec<String>) -> QrView {
        self.public = true;
        self.png_name = name.to_string();
        self.label = lines;
        self
    }

    /// Whether the sheet offers the code as a PNG: public, not a secret,
    /// and one static code.
    pub fn offers_png(&self) -> bool {
        self.public && !self.secret && self.frames.len() == 1 && !self.png_name.is_empty()
    }

    /// The code on show as a labelled picture, its file's name and
    /// bytes, when it offers one.
    pub fn png(&self) -> Option<(String, Vec<u8>)> {
        if !self.offers_png() {
            return None;
        }
        let code = self.frames.first()?;
        Some((
            self.png_name.clone(),
            picture::labelled_png(code, &self.title, &self.label),
        ))
    }
}

/// Specter's numbered parts, `part` characters of text a frame, one code
/// each; a text that fits one frame is one code with no number.
fn parts_frames(text: &str, part: usize) -> Result<Frames, String> {
    use osk_codec::qr::{Ecc, Payload, encode};
    let parts = if text.chars().count() <= part {
        vec![text.to_string()]
    } else {
        faraday_qr::specter::encode(text, part)
    };
    let frames = parts
        .iter()
        .map(|p| encode(Payload::Bytes(p.as_bytes()), Ecc::Low).map_err(|e| format!("{e:?}")))
        .collect::<Result<Vec<_>, _>>()?;
    let sub = if frames.len() == 1 {
        "Text · one code".to_string()
    } else {
        format!("Numbered parts · {} frames, shown in turn", frames.len())
    };
    Ok((sub, frames))
}

/// BBQr parts in encoding `2`, one code each.
fn bbqr_frames(t: char, data: &[u8], part: usize) -> Result<Frames, String> {
    use osk_codec::qr::{Ecc, Payload, encode};
    let parts = faraday_qr::bbqr::encode(t, data, part).ok_or("Too large for BBQr")?;
    let frames = parts
        .iter()
        .map(|p| {
            encode(Payload::Alphanumeric(p.as_bytes()), Ecc::Low).map_err(|e| format!("{e:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let sub = if frames.len() == 1 {
        format!("BBQr {t} · one code")
    } else {
        format!("BBQr {t} · {} frames, shown in turn", frames.len())
    };
    Ok((sub, frames))
}

/// Text as pictures keep it: one static code while it fits version 25
/// at medium error correction (the bound `docs/WALLETS.md` §5 sets for
/// a descriptor on screen), else the BBQr parts the QR view makes, one
/// code each, which a scanner that assembles parts takes one after the
/// other.
pub(crate) fn picture_codes(text: &str) -> Result<Vec<osk_codec::qr::QrMatrix>, String> {
    use osk_codec::qr::{Ecc, Payload, encode_bounded};
    if let Ok(m) = encode_bounded(Payload::Bytes(text.as_bytes()), Ecc::Medium, 25) {
        return Ok(vec![m]);
    }
    bbqr_frames('U', text.as_bytes(), QR_PARTS[1]).map(|(_, frames)| frames)
}

/// The line that names a cosigner's key under its code: `Key {fp} ·
/// {kind} · {path}`, the path read from the key's `[fp/path]` origin.
fn key_label(fp: &str, kind: &str, key: &str) -> String {
    let path = key
        .strip_prefix('[')
        .and_then(|k| k.split_once(']'))
        .and_then(|(origin, _)| origin.split_once('/'))
        .map(|(_, p)| format!("m/{p}"));
    match path {
        Some(p) => format!("Key {fp} · {kind} · {p}"),
        None => format!("Key {fp} · {kind}"),
    }
}

/// Bytes as `ur:bytes`: one code when it fits, else animated.
fn ur_bytes_frames(data: &[u8], what: &str, part: usize) -> Result<Frames, String> {
    use osk_codec::qr::{Ecc, Payload, encode_bounded};
    let single = osk_codec::ur::encode_bytes(data).to_ascii_uppercase();
    if let Ok(m) = encode_bounded(Payload::Alphanumeric(single.as_bytes()), Ecc::Low, 20) {
        return Ok((format!("{what} · one code"), vec![m]));
    }
    let frames = animated(
        osk_codec::ur::Encoder::bytes(data, part).map_err(|e| format!("{e:?}"))?,
        data.len(),
        part,
    )?;
    Ok((
        format!("{what} · {} frames, shown in turn", frames.len()),
        frames,
    ))
}

/// Whether an Outbox file can go as a QR transfer: what a wallet reads
/// goes as itself, anything else in the file envelope, which carries up
/// to 256 KiB. No vault is that small.
pub(crate) fn qr_fits(item: &Item) -> bool {
    match item.kind {
        FileKind::Psbt | FileKind::Wallet | FileKind::Key | FileKind::Message | FileKind::Share => {
            true
        }
        _ => !item.bytes.is_empty() && item.bytes.len() <= faraday_qr::envelope::MAX_FILE,
    }
}

/// A file's name as the envelope takes one: letters, digits, dots,
/// hyphens and underscores, starting with a letter or digit.
fn envelope_name(name: &str) -> String {
    let mut n: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .take(128)
        .collect();
    if !n.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        n.insert(0, 'f');
        n.truncate(128);
    }
    n
}

/// The frames of a fountain-coded UR: twice the fragments it takes, so a
/// scanner that misses a few still completes on the first pass.
fn animated(
    mut enc: osk_codec::ur::Encoder,
    len: usize,
    frag: usize,
) -> Result<Vec<osk_codec::qr::QrMatrix>, String> {
    use osk_codec::qr::{Ecc, Payload, encode};
    let count = (len.div_ceil(frag) * 2).max(4);
    (0..count)
        .map(|_| {
            let part = enc.next_part().to_ascii_uppercase();
            encode(Payload::Alphanumeric(part.as_bytes()), Ecc::Low).map_err(|e| format!("{e:?}"))
        })
        .collect()
}

/// What a whole transfer becomes in the Inbox: a name when it brought
/// one, an extension, and the bytes.
fn arrival(a: faraday_qr::Arrived) -> (Option<String>, &'static str, Vec<u8>) {
    use faraday_qr::Arrived;
    match a {
        Arrived::Psbt(p) => (None, "psbt", p),
        // A finished transaction is kept as hex, as one written here is.
        Arrived::Transaction(t) => (
            None,
            "txn",
            t.iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .into_bytes(),
        ),
        Arrived::File(name, data, _) => (Some(name), "", data),
        Arrived::Keys(t) | Arrived::Descriptor(t) | Arrived::Text(t) => {
            (None, "txt", t.into_bytes())
        }
    }
}

/// The progress of a multi-part code: parts read, and which are missing.
fn part_note(what: &str, have: usize, total: usize, missing: &[usize]) -> String {
    let list: Vec<String> = missing.iter().take(8).map(usize::to_string).collect();
    let more = if missing.len() > 8 { ", …" } else { "" };
    format!(
        "{what}: {have} of {total} · missing {}{more}",
        list.join(", ")
    )
}

/// Why scanned text is refused for holding a secret: a seed's words or a
/// private key.
fn secret_text(text: &str) -> Option<&'static str> {
    let t = text.trim();
    let words = t.split_whitespace().count();
    if matches!(words, 12 | 15 | 18 | 21 | 24)
        && osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, t).is_ok()
    {
        return Some("Those are a seed's words. Seeds go in through Add a key, not Files");
    }
    let key = t.split(']').next_back().unwrap_or(t);
    if [
        "xprv", "tprv", "yprv", "zprv", "Yprv", "Zprv", "uprv", "vprv", "Uprv", "Vprv",
    ]
    .iter()
    .any(|p| key.starts_with(p))
    {
        return Some(
            "That is a private key. Faraday takes keys as words or a SeedQR, in Add a key",
        );
    }
    None
}

/// What a code is when a SeedQR was wanted and something else came.
fn what_instead_of_a_seed(text: &str) -> Option<&'static str> {
    let t = text.trim();
    if wallet::read_psbt(t.as_bytes()).is_some()
        || t.to_ascii_lowercase().starts_with("ur:crypto-psbt")
    {
        return Some("That is a transaction, not a seed. Scan it from Files");
    }
    if faraday_qr::bbqr::is_part(t) || osk_codec::ur::is_ur(t) {
        return Some("That is a code for Files, not a seed");
    }
    if wallet::read_wallet(t).is_ok() || crate::create::read_key(t).is_some() {
        return Some("That is a public wallet or key, not a seed: it cannot sign");
    }
    None
}

/// Seed entry.
#[derive(Default)]
pub struct EntryState {
    /// The words as typed.
    pub typed: secret_text::SecretText,
    /// The slot fingerprint the key is meant for.
    pub wanted: Option<[u8; 4]>,
    /// The label for the key.
    pub label: String,
    /// The last refusal.
    pub error: Option<String>,
    /// Where to go once the key is added.
    pub back: Option<Screen>,
    /// The BIP-39 passphrase, when one is typed.
    pub passphrase: secret_text::SecretText,
    /// Typing goes to the passphrase rather than the words.
    pub on_passphrase: bool,
    /// Which form of key is being typed.
    pub form: forms::Form,
    /// The BIP-39 list the words are typed from, by its place in
    /// `forms::LANGUAGES`.
    pub language_at: u8,
    /// OpenSigner's word entry, for a list typed on an on-screen
    /// keyboard ([`forms::on_screen`]); `None` for a Latin list, whose
    /// words are typed into `typed`.
    pub keys: Option<Box<opensigner_core::load::LoadWizard>>,
    /// The on-screen keyboard's shift is on.
    pub shift: bool,
    /// SLIP-39 shares, codex32 strings or Seed XOR parts collected.
    pub parts: forms::Parts,
    /// The lists other than English are shown to choose from.
    pub languages: bool,
    /// The chips for SLIP-39, codex32 and Seed XOR are shown: pressed
    /// directly, or set already by a Tools tile for one of those forms
    /// (`docs/SIMPLIFY.md` §2.4).
    pub other_forms: bool,
}

impl EntryState {
    /// The BIP-39 list the words are typed from.
    pub fn language(&self) -> osk_bip::bip39::Language {
        forms::LANGUAGES[usize::from(self.language_at) % forms::LANGUAGES.len()]
    }
}

/// A transaction being signed, with its screen's state.
pub struct SpendState {
    /// The spend.
    pub spend: Spend,
    /// The open step, if any.
    pub open: Option<u8>,
    /// Steps closed as done.
    pub done: [bool; 10],
    /// The full table is shown.
    pub table: bool,
    /// The last refusal or failure.
    pub error: Option<String>,
    /// The inspection, kept current.
    pub inspection: osk_psbt::Inspection,
    /// Fingerprints with a valid signature.
    pub signers: Vec<[u8; 4]>,
    /// Which results went to the Outbox.
    pub out_signed: bool,
    /// The finished transaction went to the Outbox.
    pub out_tx: bool,
    /// The finished transaction's hex is shown.
    pub show_hex: bool,
    /// The last nonce check, with the PSBT bytes it was made over: it
    /// signs again to compare, so it runs once per set of signatures,
    /// not once per frame.
    pub(crate) nonce_check: std::cell::RefCell<Option<(Vec<u8>, std::rc::Rc<wallet::NonceCheck>)>>,
    /// The step column's scroll, in design units.
    pub scroll: f32,
    /// The open step changed: scroll it into view on the next frame.
    pub follow: bool,
    /// The loaded wallet the transaction spends from.
    pub wallet: Option<usize>,
    /// The steps this spend shows, in order (`wallet::step`).
    pub steps: Vec<u8>,
    /// Signatures needed.
    pub needed: usize,
    /// Every input can be finalised now.
    pub complete: bool,
    /// A MuSig2 round this device opened: its secret nonce, until round
    /// two. Gone on lock.
    pub musig: Option<osk_psbt::MusigSession>,
    /// A threshold spend's first location: the other shares chosen to
    /// sign, by identifier.
    pub others: Vec<u32>,
    /// The carry section a threshold spend arrived with: the secret nonce
    /// of a share held here. Wiped on drop.
    pub carry: Option<osk_psbt::threshold::CarrySection>,
    /// The carry file this device's pass leaves for the next share, until
    /// it goes to the Outbox.
    pub carry_out: Option<zeroize::Zeroizing<Vec<u8>>>,
    /// The carry it arrived with has signed: a vault's round for it goes.
    pub round_spent: bool,
    /// The open step has been set once, at entry (`refresh_spend`): once
    /// true, closing every card (an interactive collapse) leaves `open`
    /// at `None` rather than reopening one.
    pub opened: bool,
}

/// An account key of a loaded wallet held here, as
/// [`Faraday::wallet_key`] finds it.
pub struct WalletKey<'a> {
    /// The master fingerprint, as text.
    pub fp: String,
    /// `[fingerprint/path]xpub`, as the wallet's descriptor writes it.
    pub text: String,
    /// What kind of wallet it is a key of, by name.
    pub kind: String,
    /// The kind, when the key is at Create's own account for it.
    pub standard: Option<create::NewKind>,
    /// The seed's master key.
    pub master: &'a osk_bip::keys::MasterKey,
}

/// The backup flow's state.
#[derive(Default)]
pub struct BackupState {
    /// The wallet being backed up.
    pub wallet: usize,
    /// The plan, or the checklist made from it.
    pub stage: BStage,
    /// The open question of the plan, by [`qstep`].
    pub q: Option<u8>,
    /// The plan's answers.
    pub answers: plan::Answers,
    /// What each place is called, kept only in a vault; empty for
    /// "Place 1", "Place 2".
    pub names: Vec<String>,
    /// The place whose name is being typed.
    pub naming: Option<usize>,
    /// The open item of the checklist, by [`bstep`].
    pub open: Option<u8>,
    /// The envelopes closed as done: the one item nothing else marks.
    pub envelopes: bool,
    /// The descriptor was shown as a code in this backup.
    pub shown: bool,
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The session key whose seed is shown, by index into the session.
    pub key: usize,
    /// The words are shown.
    pub reveal: bool,
    /// Compact SeedQR rather than standard.
    pub compact: bool,
    /// The grid row pinned for copying.
    pub pin: usize,
    /// Typing goes to the check field.
    pub checking: bool,
    /// The digits typed back.
    pub typed: secret_text::SecretText,
    /// What scanning the copy of the seed on screen found.
    pub scanned: Option<backup::CopyCheck>,
    /// The seeds, by fingerprint, whose copy by hand matched when typed
    /// back or scanned, in this backup.
    pub checked: Vec<[u8; 4]>,
    /// The digits typed back have matched and the match is counted: a
    /// further copy is counted once Clear empties the field.
    pub counted: bool,
    /// Template word count.
    pub words: usize,
    /// What went to the Outbox, for the summary.
    pub sent: Vec<String>,
    /// Another paper form of the key, on screen.
    pub paper: Option<paper::PaperForm>,
    /// The list of loaded wallets is open under the wallet chip.
    pub pick: bool,
    /// Opened from Create's Back up card: the chip says the way on is
    /// the wallet card.
    pub from_create: bool,
    /// Opened from a chart in an open vault's view: the way back is the
    /// vault.
    pub from_vault: bool,
    /// Work done that a later plan dropped, as what it is ("vault.ofv
    /// holds seed 9A6A2580"): the copy exists until it is destroyed.
    pub extras: Vec<String>,
    /// The vaults the checklist had filled when Change the plan was
    /// pressed, by name, with the seeds each holds.
    pub prior: Vec<(String, Vec<String>)>,
}

/// Signing a message: the cards' state.
#[derive(Default)]
pub struct MessageState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; 4],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The wallet, by index into the session.
    pub wallet: Option<usize>,
    /// The receive address's index.
    pub index: u32,
    /// The message.
    pub text: String,
    /// Typing goes to the message.
    pub typing: bool,
    /// BIP-322 rather than BIP-137.
    pub bip322: bool,
    /// The signature, once made.
    pub signed: Option<osk_psbt::message::Signed>,
    /// The last refusal.
    pub error: Option<String>,
}

/// The message cards, in order.
pub mod mstep {
    /// The address.
    pub const ADDRESS: u8 = 0;
    /// The message.
    pub const TEXT: u8 = 1;
    /// The format.
    pub const FORMAT: u8 = 2;
    /// The signature.
    pub const SIGN: u8 = 3;
}

/// Creating a wallet: the cards' state.
#[derive(Default)]
pub struct CreateState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; cstep::COUNT],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The kind.
    pub kind: create::NewKind,
    /// Signatures needed.
    pub m: usize,
    /// Keys.
    pub n: usize,
    /// Where each key comes from.
    pub slots: Vec<create::Source>,
    /// The wallet, once made, by index into the session.
    pub built: Option<usize>,
    /// The last refusal.
    pub error: Option<String>,
    /// The Kind card's **More kinds** is expanded: the other eight kinds
    /// are shown too.
    pub more_kinds: bool,
    /// New keys made for this wallet.
    pub made: u32,
    /// The slot whose passphrase fields are open (`docs/NEW-WALLET.md`
    /// §3.6).
    pub pass_slot: Option<u8>,
    /// The passphrase typed there.
    pub pass: secret_text::SecretText,
    /// The same passphrase typed again.
    pub pass2: secret_text::SecretText,
    /// Which of the two fields typing goes to, if either.
    pub pass_focus: Option<u8>,
    /// Why the last Lock in did nothing.
    pub pass_note: Option<String>,
    /// Keys locked in with a passphrase here, by fingerprint: their slot
    /// shows **Locked in**.
    pub pass_locked: Vec<[u8; 4]>,
}

impl CreateState {
    /// Closes a slot's passphrase fields, forgetting what was typed.
    pub fn close_pass(&mut self) {
        self.pass_slot = None;
        self.pass.clear();
        self.pass2.clear();
        self.pass_focus = None;
        self.pass_note = None;
    }
}

/// What [`Action::About`] names for a page's walk-through rather than a
/// step's.
pub const ABOUT_PAGE: u8 = u8::MAX;

/// The create cards, in order.
pub mod cstep {
    /// The kind.
    pub const KIND: u8 = 0;
    /// The quorum.
    pub const QUORUM: u8 = 1;
    /// The keys; their Continue makes the wallet.
    pub const KEYS: u8 = 2;
    /// The descriptor and the first addresses.
    pub const CHECK: u8 = 3;
    /// The backup plan's presets, each opening Back up a wallet.
    pub const BACKUP: u8 = 4;
    /// How many cards there are.
    pub const COUNT: usize = 5;
}

/// Restoring a wallet: the cards' state (`docs/NEW-WALLET.md` §12).
#[derive(Default)]
pub struct RestoreState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; rstep::COUNT as usize],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The Inbox shares chosen, by name.
    pub shares: Vec<String>,
    /// The wallet, by index into the session: read from its description,
    /// or made from the seeds.
    pub wallet: Option<usize>,
    /// The wallet came from its description: the seeds are matched to its
    /// keys.
    pub described: bool,
    /// The last refusal.
    pub error: Option<String>,
    /// The kind, the quorum and the seeds and cosigners' xpubs in hand,
    /// for a wallet made from the seeds.
    pub seeds: Option<seeds::SeedsState>,
    /// Kind's **More kinds** is expanded.
    pub more_kinds: bool,
    /// The description's keys marked Not here, by place.
    pub absent: Vec<u8>,
    /// The open slot's passphrase, for the seed entered next.
    pub pass: secret_text::SecretText,
    /// The same, again.
    pub pass2: secret_text::SecretText,
    /// Which passphrase field typing goes to.
    pub pass_focus: Option<u8>,
    /// Opened from a key on Wallets: Back goes to Wallets.
    pub from_key: bool,
}

impl RestoreState {
    /// A fresh Restore, open on Kind with a single key ticked.
    pub fn new() -> RestoreState {
        let mut s = seeds::SeedsState::default();
        s.fix(create::NewKind::NativeSegwit);
        RestoreState {
            open: Some(rstep::KIND),
            seeds: Some(s),
            ..RestoreState::default()
        }
    }

    /// The kind chosen or read.
    pub fn kind(&self) -> create::NewKind {
        self.seeds.as_ref().map(|s| s.kind).unwrap_or_default()
    }

    /// The cards this kind has, in order: Quorum for a multisig.
    pub fn steps(&self) -> Vec<u8> {
        let kind = self.kind();
        let mut v = vec![rstep::KIND];
        if kind.multi() && !kind.threshold() {
            v.push(rstep::QUORUM);
        }
        v.extend([rstep::DESCRIPTION, rstep::SEEDS, rstep::CHECK, rstep::DONE]);
        v
    }

    /// Closes card `k` as done and opens the next not done after it.
    pub fn next(&mut self, k: u8) {
        self.done[k as usize] = true;
        let steps = self.steps();
        let at = steps.iter().position(|&s| s == k).unwrap_or(0);
        self.open = steps[at..]
            .iter()
            .chain(steps[..at].iter())
            .copied()
            .find(|&s| !self.done[s as usize]);
        self.scroll.follow = true;
    }

    /// Empties the passphrase fields.
    pub fn close_pass(&mut self) {
        self.pass.clear();
        self.pass2.clear();
        self.pass_focus = None;
    }
}

/// What Restore says of a seed that is not one of the description's keys.
pub const NOT_A_KEY: &str = "Not a key of this wallet: check the words or the passphrase";

/// The restore cards, in order (`docs/NEW-WALLET.md` §12.1).
pub mod rstep {
    /// What kind of wallet.
    pub const KIND: u8 = 0;
    /// M of N, for a multisig.
    pub const QUORUM: u8 = 1;
    /// The wallet's description, or none.
    pub const DESCRIPTION: u8 = 2;
    /// A slot per key: its seed, or a cosigner's xpub.
    pub const SEEDS: u8 = 3;
    /// The descriptor and the first addresses.
    pub const CHECK: u8 = 4;
    /// What next.
    pub const DONE: u8 = 5;
    /// How many cards.
    pub const COUNT: u8 = 6;
}

/// The backup cards, in order.
pub mod bstep {
    use crate::plan::Item;

    /// Print the blank templates.
    pub const BLANK: u8 = 0;
    /// The public files for the software chosen.
    pub const PUBLIC: u8 = 2;
    /// The wallet sheet or the shares.
    pub const SHEETS: u8 = 3;
    /// One envelope per place.
    pub const ENVELOPE: u8 = 4;
    /// The seeds as files.
    pub const FILES: u8 = 6;
    /// The descriptor shown to the watch-only software.
    pub const SHOW: u8 = 8;
    /// Copy seed i by hand and check it: `COPY + i`, the seed by its
    /// place among the wallet's, below [`VAULT`].
    pub const COPY: u8 = 16;
    /// Vault v made and filled: `VAULT + v`, the first vault `VAULT`.
    pub const VAULT: u8 = 128;

    /// Whether `n` is a seed's copy by hand.
    pub fn is_copy(n: u8) -> bool {
        (COPY..VAULT).contains(&n)
    }

    /// An item's number.
    pub fn of(item: Item) -> u8 {
        match item {
            Item::Templates => BLANK,
            Item::Copy(i) => COPY.saturating_add(i.min(usize::from(VAULT - COPY - 1)) as u8),
            Item::Vault(v) => VAULT.saturating_add(v.min(127) as u8),
            Item::SeedFiles => FILES,
            Item::Sheets => SHEETS,
            Item::PublicFiles => PUBLIC,
            Item::ShowDescriptor => SHOW,
            Item::Envelopes => ENVELOPE,
        }
    }

    /// The item a number stands for.
    pub fn item(n: u8) -> Option<Item> {
        Some(match n {
            BLANK => Item::Templates,
            FILES => Item::SeedFiles,
            SHEETS => Item::Sheets,
            PUBLIC => Item::PublicFiles,
            SHOW => Item::ShowDescriptor,
            ENVELOPE => Item::Envelopes,
            n if n >= VAULT => Item::Vault(usize::from(n - VAULT)),
            n if n >= COPY => Item::Copy(usize::from(n - COPY)),
            _ => return None,
        })
    }
}

/// The backup's two parts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BStage {
    /// The presets and the questions, with the map beside them.
    #[default]
    Plan,
    /// The checklist made from the plan.
    Checklist,
}

/// The questions of the backup's plan, in order.
pub mod qstep {
    /// The three presets.
    pub const PRESET: u8 = 0;
    /// Where the seeds go.
    pub const SEEDS: u8 = 1;
    /// The places.
    pub const PLACES: u8 = 2;
    /// Where the wallet description goes.
    pub const WALLET: u8 = 3;
    /// The watch-only software and the files' form.
    pub const SOFTWARE: u8 = 4;
    /// Where each passphrase is kept.
    pub const PASSPHRASE: u8 = 5;
    /// The map, as its own page on a small panel.
    pub const MAP: u8 = 6;
}

/// The lists a [`Action::BAnswer`] ticks, as [`plan::Question`] names
/// them.
pub mod qrow {
    /// Where the seeds go.
    pub const SEEDS: u8 = 0;
    /// How many places: the row is the count.
    pub const PLACES: u8 = 1;
    /// The whole sheet (row 0) or a share each (row 1).
    pub const SPLIT: u8 = 2;
    /// Where the wallet description goes.
    pub const WALLET: u8 = 4;
    /// The software.
    pub const SOFTWARE: u8 = 5;
    /// The form.
    pub const FORM: u8 = 6;
    /// Seed i's passphrase: `PASS + i`, a row per place, then the vault.
    pub const PASS: u8 = 16;
    /// A stick with vault v: `STICKS + v`, a row per place.
    pub const STICKS: u8 = 128;
    /// The seeds vault v holds: `VAULTS + v`, a row per seed of the
    /// wallet.
    pub const VAULTS: u8 = 192;

    /// The question a list number stands for.
    pub fn question(n: u8) -> Option<crate::plan::Question> {
        use crate::plan::Question as Q;
        Some(match n {
            SEEDS => Q::Seeds,
            PLACES => Q::Places,
            SPLIT => Q::Split,
            WALLET => Q::Wallet,
            SOFTWARE => Q::Software,
            FORM => Q::Form,
            n if n >= VAULTS => Q::Vault(usize::from(n - VAULTS)),
            n if n >= STICKS => Q::Sticks(usize::from(n - STICKS)),
            n if n >= PASS => Q::Passphrase(usize::from(n - PASS)),
            _ => return None,
        })
    }
}

/// A column of the stick visit's two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    /// Write from the Outbox.
    Outbox,
    /// Import into the Inbox: the stick's files.
    Stick,
}

/// The visit screen's choices.
#[derive(Default)]
pub struct VisitState {
    /// The stick shown, by index.
    pub stick: usize,
    /// Outbox files chosen to write, by name.
    pub out: BTreeSet<String>,
    /// Stick files chosen to copy, by name.
    pub inn: BTreeSet<String>,
    /// The settings file's row as the person set it; `None` for its
    /// default (`Faraday::visit_settings_on`).
    pub settings: Option<bool>,
    /// What the last write or copy did, one line per file.
    pub log: Vec<(String, bool)>,
    /// Names from an "Import and load" copy, by name: loaded as keys the
    /// moment the stick is pulled, no further press needed.
    pub load_after: BTreeSet<String>,
    /// How far the Outbox list is scrolled, design units. The stick's
    /// files scroll by [`Faraday::list_offset`].
    pub out_offset: f32,
    /// Where each column's scrollbar was drawn, by [`VisitState::bar`].
    pub bars: [std::cell::Cell<Option<VisitBarAt>>; 2],
    /// Inbox files chosen to write, by name: a copy from one stick to
    /// another. One that may be a secret is chosen only on the secret
    /// sheet.
    pub from_inbox: BTreeSet<String>,
    /// Inbox files sent to be written and not yet answered, by name.
    pub writing_inbox: BTreeSet<String>,
    /// The write under way: the stick's label, and each file sent with
    /// the SHA-256 of its bytes. Its first answer starts a new receipt.
    pub writing: Option<Writing>,
}

/// A write under way: the stick's label, each file sent with its
/// SHA-256, and whether an answer has started its receipt.
pub type Writing = (String, Vec<(String, [u8; 32])>, bool);

/// What the last stick visit wrote (`docs/SIMPLIFY.md` §4.3): kept on
/// [`Faraday`], across a lock in the kept state, gone at power-off, and
/// replaced by the next write. Nothing on it is written again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Receipt {
    /// The stick's label.
    pub label: String,
    /// When the write was answered, Unix seconds, when the clock is known.
    pub time: Option<u64>,
    /// One row per file sent.
    pub files: Vec<ReceiptFile>,
}

/// One file on a [`Receipt`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptFile {
    /// Its name.
    pub name: String,
    /// SHA-256 of the bytes written.
    pub hash: [u8; 32],
    /// Written, read back and matched.
    pub verified: bool,
    /// Why it was not, when it was not.
    pub failure: Option<String>,
}

impl Receipt {
    /// "Written to STICK at 14:02", or without the time when the clock
    /// is not known.
    pub fn heading(&self) -> String {
        match self.time {
            Some(t) => format!("Written to {} at {}", self.label, vaults::time_of_day(t)),
            None => format!("Written to {}", self.label),
        }
    }

    /// The file written and matched under `name` with these bytes.
    pub fn holds(&self, name: &str, bytes: &[u8]) -> bool {
        let hash = sha256_of(bytes);
        self.files
            .iter()
            .any(|f| f.verified && f.name == name && f.hash == hash)
    }

    /// Whether `name` was written and matched.
    pub fn wrote(&self, name: &str) -> bool {
        self.files.iter().any(|f| f.verified && f.name == name)
    }

    /// As the kept state holds it: a `receipt` line, then a `file` line
    /// per file, fields split by tabs.
    pub(crate) fn encode(&self) -> Vec<u8> {
        let field = |s: &str| s.replace(['\t', '\n', '\r'], " ");
        let mut out = format!(
            "receipt\t{}\t{}\n",
            field(&self.label),
            self.time.map(|t| t.to_string()).unwrap_or_default()
        );
        for f in &self.files {
            let hash: String = f.hash.iter().map(|b| format!("{b:02x}")).collect();
            out.push_str(&format!(
                "file\t{}\t{hash}\t{}\t{}\n",
                field(&f.name),
                u8::from(f.verified),
                field(f.failure.as_deref().unwrap_or(""))
            ));
        }
        out.into_bytes()
    }

    /// Read back; `None` when it does not read.
    pub(crate) fn decode(bytes: &[u8]) -> Option<Receipt> {
        let text = String::from_utf8_lossy(bytes);
        let mut out: Option<Receipt> = None;
        for line in text.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            match f[..] {
                ["receipt", label, time] => {
                    out = Some(Receipt {
                        label: label.to_string(),
                        time: time.parse().ok(),
                        files: Vec::new(),
                    });
                }
                ["file", name, hash, verified, failure] => {
                    let bytes: Option<Vec<u8>> = (0..32)
                        .map(|i| {
                            hash.get(i * 2..i * 2 + 2)
                                .and_then(|h| u8::from_str_radix(h, 16).ok())
                        })
                        .collect();
                    let Some(hash) = bytes.and_then(|b| <[u8; 32]>::try_from(b).ok()) else {
                        continue;
                    };
                    if let Some(r) = out.as_mut() {
                        r.files.push(ReceiptFile {
                            name: name.to_string(),
                            hash,
                            verified: verified == "1",
                            failure: (!failure.is_empty()).then(|| failure.to_string()),
                        });
                    }
                }
                _ => {}
            }
        }
        out
    }
}

/// SHA-256 of `bytes`.
pub(crate) fn sha256_of(bytes: &[u8]) -> [u8; 32] {
    use osk_bip::bitcoin::hashes::{Hash, sha256};
    sha256::Hash::hash(bytes).to_byte_array()
}

/// Where a stick visit column's scrollbar was drawn, in pixels: the
/// track's top and height, the thumb's height, and the furthest the list
/// scrolls in design units.
pub type VisitBarAt = (i32, i32, i32, f32);

impl VisitState {
    /// Where `column`'s scrollbar was drawn, `None` when its list fits.
    pub fn bar(&self, column: Column) -> &std::cell::Cell<Option<VisitBarAt>> {
        &self.bars[match column {
            Column::Outbox => 0,
            Column::Stick => 1,
        }]
    }
}

/// The app.
pub struct Faraday {
    canvas: Option<Canvas>,
    f: f32,
    w: f32,
    h: f32,
    /// The display is a small panel: under 600 dp wide, laid out one
    /// column, pages in place of the sidebar (`Faraday::is_compact`).
    compact: bool,
    hits: Vec<(Rect, Action)>,
    pressed: Option<(Action, Rect)>,
    commands: VecDeque<Command>,
    storage_out: VecDeque<StorageCommand>,
    dirty: bool,
    restart: bool,
    last_display: Option<DisplayInfo>,
    /// The person's display scale, in percent of the automatic one.
    pub scale_pct: u16,
    /// The palette the screens are drawn in.
    pub theme: ui::Theme,
    /// Reduce motion: nothing slides, glides, coasts or stretches.
    pub reduce_motion: bool,
    /// Minutes without input before the session locks; 0 for never
    /// (`PLAN.md` §12.3).
    pub idle_lock_min: u16,
    /// How long each frame of an animated code stays up, in ms.
    pub qr_frame_ms: u64,
    /// Minutes without input before the machine powers off, when nothing
    /// is held and the Outbox is empty; 0 for never.
    pub idle_off_min: u16,
    /// When the last input came, on this process's clock; set at the
    /// first tick.
    last_input: Option<u64>,
    /// Idle time the previous process had when it locked for idleness,
    /// counted toward power-off here.
    idle_carry: u64,
    /// This process locks for idleness, after this many ms without input.
    idle_locking: Option<u64>,
    /// The idle countdown's whole seconds last drawn, so a tick redraws
    /// only when the number on screen changes.
    idle_shown: Option<u64>,
    /// Whole minutes without input when the idle warning opened: what
    /// its text says, fixed while it is up.
    pub(crate) idle_warn_min: u64,
    /// A secret has been held or typed in this process. Only a fresh
    /// process is clean (`PLAN.md` §5.1).
    tainted: bool,
    /// A stick arrived while the camera was on: held back, and taken as
    /// arriving once the camera is off.
    stick_held: bool,
    /// New input devices waiting to be believed (`PLAN.md` §4.6).
    pub inputs: Vec<inputs::NewInput>,
    /// Devices kept out until unplugged, by id and name.
    pub ignored_inputs: Vec<(u32, String)>,
    /// The boot import's sheet waits under the new-device sheet, and
    /// comes up once every waiting device is decided.
    import_under_input: bool,
    /// Decisions the shell has not taken yet.
    input_decisions: VecDeque<(u32, bool)>,
    /// New keys made this session, which number their labels.
    new_keys: u32,
    /// The keys made here whose backup is not yet done, by fingerprint
    /// (`docs/NEW-WALLET.md` §14.3): they sign nothing until it is. Never
    /// a secret; kept in a vault beside the seed and in the plan record.
    made_here: Vec<osk_bip::keys::Fingerprint>,
    /// The vaults: the open slots and the vault screens' state.
    pub vaults: vaults::Vaults,
    /// The amounts each signed transaction's inputs stated
    /// (`docs/WALLETS.md` §3.3).
    pub signed_amounts: Vec<memory::Signed>,
    /// Seal the signed-amount memory into an open vault on lock.
    pub seal_amounts: bool,
    /// The wallets, by descriptor checksum, a person has pressed
    /// Continue on Check (Sign a transaction, Create) or "It matches"
    /// (Spend tab) for, this power-on, kept across a lock
    /// (`docs/WALLETS.md` §4).
    pub checked_wallets: Vec<String>,
    /// What this power-on saw of each wallet's backup, by descriptor
    /// checksum (`docs/SIMPLIFY.md` §5): kept across a lock, gone at
    /// power-off.
    pub backup_records: Vec<backups::Record>,
    /// The wallet the Backups screen is on, by its place in
    /// [`Faraday::backups`].
    pub backups_at: usize,
    /// What the last stick visit wrote, this power-on (§4.3).
    pub receipt: Option<Receipt>,
    /// Each vault file read from a stick this power-on: its name, the
    /// hash of its bytes as read, and the stick's label (§3.5).
    pub vault_from: Vec<(String, [u8; 32], String)>,
    seed: [u8; 32],
    /// The session's seed has arrived.
    seeded: bool,
    /// Signing passes that drew a seed of their own (`fresh.rs`).
    sign_draws: u64,
    /// Fresh answers waiting for the next key (`fresh.rs`).
    pool: Vec<zeroize::Zeroizing<[u8; 32]>>,
    now_ms: u64,
    toast: Option<(String, u64)>,

    /// The screen shown.
    pub screen: Screen,
    /// A sheet over it.
    pub sheet: Option<Sheet>,
    /// How far a sheet taller than a small panel is scrolled, and which
    /// sheet it was: another sheet starts at its top.
    sheet_scroll: (f32, Option<Sheet>),
    /// A small panel's docked keyboard.
    osk: compact::Osk,
    /// The Wallets tab's session.
    pub session: Session,
    /// The transaction being signed.
    pub spend: Option<SpendState>,
    /// The transaction being decoded.
    pub decode: Option<DecodeState>,
    /// What is typed in Tools' Find a tool.
    pub catalog_find: String,
    /// Files read from sticks.
    pub inbox: Vec<Item>,
    /// Results waiting for a stick.
    pub outbox: Vec<Item>,
    /// The sticks present.
    pub sticks: Vec<StickInfo>,
    /// "Not now" was chosen for the sticks present.
    pub not_now: bool,
    /// What was pressed with a stick attached, done when it is pulled
    /// ([`Sheet::Pull`]).
    pub pull: Option<Action>,
    /// The flow a stick visit returns to when the stick is pulled.
    pub after_visit: Option<Screen>,
    /// Seed entry.
    pub entry: EntryState,
    /// The wallet chosen on the Wallets tab.
    pub wallet: usize,
    /// The key in no wallet chosen on the Wallets tab, by fingerprint,
    /// when several are loaded and none of them in a wallet.
    pub loose_pick: Option<[u8; 4]>,
    /// The visit screen.
    pub visit: VisitState,
    /// What the boot stick brought at its first look this power-on,
    /// waiting to be imported (`boot_import`).
    pub import: Option<boot_import::ImportState>,
    /// The settings the boot stick holds, as `settings_body` writes them:
    /// read from it at boot, or written to it since; `None` until the
    /// boot stick has been looked at (`stick_settings`).
    pub stick_settings: Option<String>,
    /// Guided mode: flows show their written walk-through.
    pub guided: bool,
    /// On a small panel, the height of the pinned bar the last frame
    /// drew: the page is laid out above it.
    pub(crate) pin_h: std::cell::Cell<f32>,
    /// On a small panel, the walk-through shown: the screen, and the
    /// step's place or [`ABOUT_PAGE`]. Behind a tap there, so the
    /// controls keep the panel.
    pub about_open: Option<(Screen, u8)>,
    /// Pixels scrolled past in the long list on screen (the stick's
    /// files, Files, the wallet list): continuous, not row-snapped, so a
    /// wheel notch moves the content by its own pixels like everywhere
    /// else.
    pub list_offset: f32,
    /// Design units scrolled past in the wallet card's body, on a desktop.
    pub card_offset: f32,
    /// Design units scrolled past in Vault contents' item, on a desktop.
    pub vault_item_offset: f32,
    /// The wallet's chart is open as its own page, on a small panel.
    pub glance: bool,
    /// The chart and the thing on it whose sheet is open.
    pub chart: Option<(glance::Press, glance_sheet::Target)>,
    /// A chart's key whose lines are drawn bold, by place among its keys:
    /// **Where it is**.
    pub chart_focus: Option<(glance::Press, usize)>,
    /// What the chart's sheets hold between presses: a change asked for,
    /// a name or a passphrase typed (`docs/NEW-WALLET.md` §9.5–§9.7).
    pub chart_work: chart_edit::Work,
    /// The height a scrolled page last drew to, design units: how far its
    /// scroll may go.
    pub(crate) content_h: std::cell::Cell<f32>,
    /// What moves the scrolled region: pans, glides, coasts and the
    /// stretch at its ends (`docs/MOTION.md` §3.3).
    motion: motion::Motion,
    /// The screen, sheet and region the motion belongs to.
    motion_for: RegionKey,
    /// The scrolled regions the last frame drew, and the screen and
    /// sheet they were drawn for.
    extent: Option<(ScreenKey, Vec<ui::Scrolled>)>,
    /// The region the wheel, a trackpad and a finger last went to: the
    /// one under the pointer when it started.
    region: ui::Slot,
    /// For the overlay scrollbar: the screen, sheet and region, the
    /// offset last drawn, and when it last changed (0 for not since it
    /// opened).
    bar_seen: (RegionKey, f32, u64),
    /// The QR sheet's contents.
    pub qr: Option<QrView>,
    /// The Spend tab.
    pub family: family::FamilyState,
    /// A vanity address being searched for.
    pub vanity: Option<vanity::VanityState>,
    /// The backup flow.
    pub backup: Option<BackupState>,
    /// Signing a message.
    pub message: Option<MessageState>,
    /// The Inbox file of the signed message being checked.
    pub checking: Option<usize>,
    /// Creating a wallet.
    pub create: Option<CreateState>,
    /// Restoring a wallet.
    pub restore: Option<RestoreState>,
    /// A new key being made.
    pub keygen: Option<keygen::KeyGen>,
    /// The Learn sheet.
    pub learn: learn::LearnState,
    /// The word-list sheet, while it is open.
    pub wordlist: Option<wordlist::WordListState>,
    /// BIP-85 being derived.
    pub bip85: Option<bip85::Bip85State>,
    /// Silent payments on show.
    pub silent: Option<silent::SilentState>,
    /// Explore on show.
    pub explore: Option<explore::ExploreState>,
    /// The Lightning node key on show.
    pub lightning: Option<lightning::LightningState>,
    /// Tools on show.
    pub tools: Option<tools::ToolsState>,
    /// A secret waiting for the person to say where it goes.
    pub secret_out: Option<secrets::SecretOut>,
    /// The person has read what that secret gives away.
    pub secret_ack: bool,
    /// The camera is on for camera noise.
    keygen_camera_on: bool,
    /// A transfer in parts the camera was closed on, kept for the next
    /// scan.
    scan_parts: Option<(osk_codec::ur::Decoder, faraday_qr::Assembler)>,
    /// The cameras the shell can open. With none, the backup's seeds
    /// step checks a copy by its typed numbers instead of the camera.
    pub cameras: Vec<(String, String)>,
    /// The one chosen, by id; the shell's first when none is.
    pub camera: Option<String>,
    /// A camera chosen that the shell has not opened yet.
    camera_change: Option<String>,
    /// The camera sheet.
    pub scan: Option<ScanState>,
    /// Transfer, in the online app.
    pub transfer: transfer::TransferState,
    /// The upgrade, while it is on screen.
    pub upgrade: Option<upgrade::UpgradeState>,
    /// The lock sheet is up for the upgrade: the fresh process opens on
    /// it.
    pub upgrade_after_lock: bool,
    /// The chosen wallet's new name, while it is typed.
    pub renaming: Option<String>,
    /// Where the caret and the selection are in the field typing goes
    /// to (`docs/NEW-WALLET.md` §13.2).
    pub(crate) edit: ui::Edit,
    /// An edit inside a field is being typed through the field's own
    /// keys.
    editing: bool,
    /// Shift is held.
    shift: bool,
    /// Where the last frame drew each line of typing's characters.
    fields: Vec<ui::FieldAt>,
    /// The screen's back link, as the last frame drew it.
    back_link: Option<Action>,
    /// Screens left by the mouse's back button, the last left last
    /// (`docs/NEW-WALLET.md` §13.1).
    forward: Vec<Screen>,
    /// The screen the last back or forward button landed on.
    landed: Option<Screen>,
    /// Seeds' fingerprints with no passphrase, by a hash of the secret:
    /// the Inbox's wallets and seeds are put together on every frame.
    pub(crate) fp_memo: std::cell::RefCell<Vec<([u8; 32], osk_bip::keys::Fingerprint)>>,
    /// Wallets the Inbox describes left out of its next load, by
    /// descriptor.
    pub inbox_skip: BTreeSet<String>,
    /// A seed from the Inbox being made a wallet, or a sealed backup
    /// being opened.
    pub potential: Option<inbox::Potential>,
    /// The last tap that acted and when, for a double-click.
    last_tap: Option<(Action, u64)>,
    /// Where the finger or pointer went down, in pixels.
    down_at: (i32, i32),
    /// A finger on the scrolled region: the row it was last at, and
    /// whether it has moved far enough to be scrolling rather than
    /// pressing.
    drag: Option<(i32, bool)>,
    /// The touch now down only stopped a coast, and does nothing else.
    swallow: bool,
    /// The overlay scrollbar is held: how far down its thumb, in pixels,
    /// the finger or pointer took it.
    bar_held: Option<i32>,
    /// The tick the text caret's blink last started over at: a key or a
    /// press, so it shows while typing goes on.
    caret_at: u64,
    /// The last frame drew a caret, and whether it showed.
    caret_drawn: Option<bool>,
    /// The tick the last two-finger scroll came in at, until its end.
    scroll_at: Option<u64>,
    /// Where the pointer is, with no button down, in pixels.
    hover: Option<(i32, i32)>,
    /// The tick the toast on show first came up at.
    toast_at: Option<u64>,
    /// The step column's open card, and one opening or closing.
    disclosed: Option<Disclosed>,
    /// The frosted page under the open sheet, and the screen, sheet and
    /// theme it was made for.
    frost: Option<((ScreenKey, ui::Theme), Vec<u8>)>,
    /// The Guided switch's pill sliding: where it started, 0 on Steps
    /// only and 1 on Guided, and the tick it started at.
    guided_moving: Option<(f32, Option<u64>)>,
    /// What the start-up self-test found (`osk_selftest`): run when the
    /// display first arrives, before any input. A failure blocks the app
    /// on a screen naming the check, with Exit its only control.
    selftest: Option<osk_selftest::Outcome>,
    /// What can be pressed under it.
    hovered: Option<Action>,
    scanned: u32,
    /// Running on an online machine (the desktop app), where sheets are
    /// turned into PDFs. Never set on the device.
    pub online: bool,
    /// What the removable medium is called and drawn as: a stick, or an
    /// SD card on the Pi. The shell sets it at start.
    pub medium: Medium,
    /// The online app's warning that this computer is not air-gapped has
    /// been acknowledged this session: it comes before mainnet does.
    airgap_warned: bool,
    /// The mainnet network asked for while that warning is on show.
    mainnet_asked: Option<osk_bip::keys::Network>,
}

impl Default for Faraday {
    fn default() -> Self {
        Self::new()
    }
}

/// The session's own seed for fresh randomness goes with the app; the
/// rest of what it holds wipes itself as it drops.
impl Drop for Faraday {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.seed);
    }
}

impl Faraday {
    /// A fresh app: no keys, empty boxes, Home.
    pub fn new() -> Self {
        Faraday {
            canvas: None,
            f: 1.0,
            w: 1280.0,
            h: 800.0,
            compact: false,
            hits: Vec::new(),
            pressed: None,
            commands: VecDeque::new(),
            storage_out: VecDeque::new(),
            dirty: true,
            restart: false,
            last_display: None,
            scale_pct: 100,
            theme: ui::Theme::default(),
            reduce_motion: false,
            idle_lock_min: IDLE_LOCK_MIN,
            qr_frame_ms: QR_SPEEDS[0],
            idle_off_min: IDLE_OFF_MIN,
            last_input: None,
            idle_carry: 0,
            idle_locking: None,
            idle_shown: None,
            idle_warn_min: 0,
            tainted: false,
            stick_held: false,
            inputs: Vec::new(),
            ignored_inputs: Vec::new(),
            import_under_input: false,
            input_decisions: VecDeque::new(),
            new_keys: 0,
            made_here: Vec::new(),
            vaults: vaults::Vaults::default(),
            signed_amounts: Vec::new(),
            seal_amounts: true,
            checked_wallets: Vec::new(),
            backup_records: Vec::new(),
            backups_at: 0,
            receipt: None,
            vault_from: Vec::new(),
            seed: [0; 32],
            seeded: false,
            sign_draws: 0,
            pool: Vec::new(),
            now_ms: 0,
            toast: None,
            screen: Screen::Home,
            sheet: None,
            sheet_scroll: (0.0, None),
            osk: compact::Osk::default(),
            session: Session::default(),
            spend: None,
            decode: None,
            catalog_find: String::new(),
            inbox: Vec::new(),
            outbox: Vec::new(),
            sticks: Vec::new(),
            not_now: false,
            pull: None,
            after_visit: None,
            entry: EntryState::default(),
            wallet: 0,
            loose_pick: None,
            visit: VisitState::default(),
            import: None,
            stick_settings: None,
            // A device that has never had a setting saved opens Guided;
            // Steps only is a choice (`docs/SIMPLIFY.md` §6.1). A saved
            // value, from the kept state or the boot stick's settings
            // file, overwrites this before the person sees a screen.
            guided: true,
            about_open: None,
            pin_h: std::cell::Cell::new(0.0),
            list_offset: 0.0,
            card_offset: 0.0,
            vault_item_offset: 0.0,
            glance: false,
            chart: None,
            chart_focus: None,
            chart_work: chart_edit::Work::default(),
            content_h: std::cell::Cell::new(0.0),
            motion: motion::Motion::default(),
            motion_for: ((Screen::Home, None), ui::Slot::Page),
            extent: None,
            region: ui::Slot::Page,
            bar_seen: (((Screen::Home, None), ui::Slot::Page), 0.0, 0),
            qr: None,
            family: family::FamilyState::default(),
            vanity: None,
            backup: None,
            message: None,
            checking: None,
            create: None,
            restore: None,
            keygen: None,
            learn: learn::LearnState::default(),
            wordlist: None,
            bip85: None,
            silent: None,
            explore: None,
            lightning: None,
            tools: None,
            secret_out: None,
            secret_ack: false,
            keygen_camera_on: false,
            scan_parts: None,
            cameras: Vec::new(),
            camera: None,
            camera_change: None,
            scan: None,
            transfer: transfer::TransferState::default(),
            upgrade: None,
            upgrade_after_lock: false,
            scanned: 0,
            renaming: None,
            edit: ui::Edit::default(),
            editing: false,
            shift: false,
            fields: Vec::new(),
            back_link: None,
            forward: Vec::new(),
            landed: None,
            fp_memo: Default::default(),
            inbox_skip: BTreeSet::new(),
            potential: None,
            last_tap: None,
            down_at: (0, 0),
            drag: None,
            swallow: false,
            bar_held: None,
            caret_at: 0,
            caret_drawn: None,
            scroll_at: None,
            hover: None,
            toast_at: None,
            disclosed: None,
            frost: None,
            guided_moving: None,
            selftest: None,
            hovered: None,
            online: false,
            medium: Medium::Stick,
            airgap_warned: false,
            mainnet_asked: None,
        }
    }

    /// The camera the person chose since the last call, by the id
    /// [`StorageEvent::Cameras`] gave it, for the shell to open.
    pub fn take_camera(&mut self) -> Option<String> {
        self.camera_change.take()
    }

    /// Whether the last exit asked for a fresh process.
    pub fn restart_requested(&self) -> bool {
        self.restart
    }

    /// The next storage request, or `None`.
    pub fn poll_storage(&mut self) -> Option<StorageCommand> {
        self.storage_out.pop_front()
    }

    /// Delivers what the shell knows about storage.
    pub fn storage(&mut self, event: StorageEvent) {
        self.storage_event(event);
        self.backup_settle();
        // A device still waiting comes back up once no other sheet is.
        self.input_sheet();
    }

    fn storage_event(&mut self, event: StorageEvent) {
        let event = match self
            .settings_event(event)
            .and_then(|e| self.import_event(e))
            .and_then(|e| self.upgrade_event(e))
        {
            Some(e) => e,
            None => {
                self.dirty = true;
                self.commands.push_back(Command::Draw);
                return;
            }
        };
        match event {
            StorageEvent::Sticks(sticks) => self.sticks_changed(sticks),
            StorageEvent::Read { stick, name, bytes } => {
                // A file Faraday reads as nothing else still comes in, as
                // a File: to sign, or to send as codes.
                let item = Item::new(&name, bytes);
                if item.kind == FileKind::Vault
                    && let Some(s) = self.sticks.iter().find(|s| s.id == stick)
                {
                    let label = s.label.clone();
                    self.vault_read_from(&name, &item.bytes, &label);
                }
                let line = if item.kind == FileKind::Words && self.visit.load_after.contains(&name)
                {
                    format!(
                        "Copied {name}: a key's words, loaded when the {} is removed",
                        self.medium.noun()
                    )
                } else {
                    format!("Copied {name}")
                };
                self.inbox.retain(|i| i.name != name);
                self.inbox.push(item);
                self.visit.log.push((line, true));
                self.visit.inn.remove(&name);
                self.save_boxes();
            }
            StorageEvent::ReadFailed { name, reason, .. } => {
                self.visit.log.push((format!("{name}: {reason}"), false));
            }
            StorageEvent::Written { name, wrote_as, .. } => {
                self.receipt_add(&name, None);
                // A copy from the Inbox stays there, for the next stick.
                if self.visit.writing_inbox.remove(&name) {
                    self.visit.from_inbox.remove(&name);
                } else {
                    // Written, it leaves For the stick for the receipt. A
                    // vault stays in Files as the stick now holds it, to
                    // unlock again; its copy from before goes.
                    if let Some(k) = self.outbox.iter().position(|i| i.name == name) {
                        let item = self.outbox.remove(k);
                        if item.kind == FileKind::Vault
                            && let Ok(h) = faraday_vault::read_header(&item.bytes)
                        {
                            self.inbox.retain(|i| {
                                i.name != item.name
                                    && !(i.kind == FileKind::Vault
                                        && faraday_vault::read_header(&i.bytes)
                                            .is_ok_and(|o| o.salt == h.salt))
                            });
                            self.inbox.push(Item::new(&item.name, item.bytes.clone()));
                        }
                    }
                    self.visit.out.remove(&name);
                }
                let line = if wrote_as == name {
                    format!("Wrote {name}, read back and matched")
                } else {
                    format!("Wrote {name} as {wrote_as}, read back and matched")
                };
                self.visit.log.push((line, true));
                self.save_boxes();
            }
            StorageEvent::WriteFailed { name, reason, .. } => {
                self.receipt_add(&name, Some(reason.clone()));
                let stays = if self.visit.writing_inbox.remove(&name) {
                    "It stays in Files".to_string()
                } else {
                    format!("It still waits {}", self.medium.for_the())
                };
                self.visit
                    .log
                    .push((format!("{name} not written: {reason}. {stays}"), false));
            }
            StorageEvent::Printed { path } => self.toast(&format!("Saved {path}")),
            StorageEvent::Clock { unix_secs } => {
                self.vaults.unix_secs = Some(unix_secs);
                self.vaults.clock_at_ms = self.now_ms;
                return;
            }
            StorageEvent::Memory { available_mib } => {
                self.vaults.memory_free_mib = Some(available_mib);
                if !matches!(self.screen, Screen::CreateVault | Screen::Unlock) {
                    return;
                }
            }
            StorageEvent::Cameras(list) => {
                // Sent as the shell looks again: drawn again only when it
                // changes, since what the copy item offers depends on it.
                if self.cameras == list {
                    return;
                }
                self.cameras = list;
            }
            StorageEvent::QrSeen {
                width,
                height,
                corners,
                module_tenths,
                read,
            } => {
                if let Some(s) = self.scan.as_mut() {
                    let now = s.frames;
                    s.seen = Some((
                        Seen {
                            width,
                            height,
                            corners,
                            read,
                        },
                        now,
                    ));
                    if !read && module_tenths < TOO_FINE_TENTHS {
                        s.too_fine += 1;
                    } else {
                        s.too_fine = 0;
                    }
                    if s.too_fine >= TOO_FINE_PASSES {
                        s.note = Some(
                            "Too fine for this camera: show it as an animated code with smaller \
                             parts, bring it closer, or carry it as a file"
                                .to_string(),
                        );
                    }
                }
            }
            StorageEvent::QrRead {
                name,
                bytes,
                payloads,
            } => {
                self.visit.inn.remove(&name);
                // The picture itself comes in as it is, beside what its
                // codes hold, to be written to another stick; it takes
                // the exposure of what they hold.
                let before = self.inbox.len();
                self.qr_read(&name, payloads);
                let holds = self.picture_holds(before);
                // Under its own name, unless what its codes held took it.
                let got = if self.inbox.iter().skip(before).any(|i| i.name == name) {
                    self.free_inbox_name(&name)
                } else {
                    self.inbox.retain(|i| i.name != name);
                    name
                };
                let mut item = Item::new(&got, bytes);
                item.picture = Some(holds);
                self.inbox.push(item);
                self.save_boxes();
            }
            StorageEvent::PrintFailed { reason } => self.toast(&format!("Not printed: {reason}")),
            StorageEvent::HostFiles { dir, files, opens } => {
                if !self.transfer_listed(dir, files, opens) {
                    return;
                }
            }
            StorageEvent::Dropped { path, size } => self.transfer_dropped(path, size),
            StorageEvent::HostRead { path, bytes } => self.transfer_read(path, bytes),
            StorageEvent::HostReadFailed { path, reason } => {
                self.transfer_read_failed(path, reason);
            }
            StorageEvent::Saved { path } => self.transfer_saved(Ok(path)),
            StorageEvent::SaveFailed { reason } => self.transfer_saved(Err(reason)),
            StorageEvent::NewInput {
                id,
                name,
                keyboard,
                pointer,
            } => self.input_new(id, name, keyboard, pointer),
            StorageEvent::InputTyped { id, ch } => self.input_typed(id, ch),
            StorageEvent::InputGone { id } => self.input_gone(id),
            // The boot copier's answers, taken by `upgrade_event`.
            StorageEvent::Boots(_)
            | StorageEvent::BootSource { .. }
            | StorageEvent::BootSourceFailed { .. }
            | StorageEvent::BootWritten { .. }
            | StorageEvent::BootWriteFailed { .. } => {}
            StorageEvent::Restored {
                inbox,
                outbox,
                kept,
            } => {
                self.inbox = inbox.into_iter().map(|(n, b)| Item::new(&n, b)).collect();
                self.outbox = outbox.into_iter().map(|(n, b)| Item::new(&n, b)).collect();
                self.restore_kept(&kept);
            }
        }
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    /// Whether anything secret is being typed: seed words, a BIP-39 or
    /// vault passphrase, a prompt's line.
    fn secret_typed(&self) -> bool {
        self.keygen.is_some()
            || self.secret_out.is_some()
            || self.lightning.is_some()
            || !self.entry.typed.is_empty()
            || !self.entry.passphrase.is_empty()
            || !self.vaults.passphrase.text.is_empty()
            || self
                .potential
                .as_ref()
                .is_some_and(|p| !p.passphrase.is_empty())
            || self
                .vaults
                .prompt
                .as_ref()
                .is_some_and(|p| !p.text.text.is_empty())
            || self.vaults.create.as_ref().is_some_and(|c| {
                c.phrases
                    .iter()
                    .any(|(a, b)| !a.text.is_empty() || !b.text.is_empty())
            })
    }

    fn note_secrets(&mut self) {
        if self.holds_secret() || self.secret_typed() {
            self.tainted = true;
        }
    }

    /// The clean state of `PLAN.md` §5.1: this process has never held or
    /// typed a secret. Only then may a stick be handed to the disk
    /// process; the shell publishes it for `faraday-grant`, and a lock is
    /// the way back to it.
    ///
    /// While the camera is on the process is not clean either, since a
    /// frame may picture a seed: no stick is handed out then, and one
    /// plugged in waits until the camera is off.
    pub fn clean(&mut self) -> bool {
        self.note_secrets();
        !self.tainted && !self.camera_on()
    }

    /// The camera is on, reading codes.
    ///
    /// Transfer's Receive, in the online app, does not count: that app is
    /// never clean of the network, and no stick waits on its camera.
    fn camera_on(&self) -> bool {
        let transfer = self
            .scan
            .as_ref()
            .is_some_and(|s| s.purpose == ScanPurpose::Transfer);
        (self.sheet == Some(Sheet::Scan) && !transfer) || self.keygen_camera_on
    }

    /// A stick held back while the camera was on arrives once it is off.
    fn release_held_stick(&mut self) {
        if !self.stick_held || self.camera_on() {
            return;
        }
        self.stick_held = false;
        if !self.sticks.is_empty() {
            self.stick_arrived();
            self.dirty = true;
            self.commands.push_back(Command::Draw);
        }
    }

    /// Input arrived: the idle timers start again.
    fn input_now(&mut self) {
        self.last_input = Some(self.now_ms);
        self.idle_carry = 0;
    }

    /// The idle time this process locked after, when it locked for
    /// idleness.
    pub(crate) fn idle_locked_after(&self) -> Option<u64> {
        self.idle_locking
    }

    /// This process follows a lock for idleness after `ms` without input:
    /// the time counts toward power-off, and the lock screen shows.
    pub(crate) fn idle_locked(&mut self, ms: u64) {
        self.idle_carry = ms;
        self.last_input = None;
        self.sheet = Some(Sheet::Locked);
    }

    /// Whether locking would wipe anything: a key, a vault, a wallet or a
    /// spend.
    fn lockable(&self) -> bool {
        self.holds_secret()
            || !self.vaults.open.is_empty()
            || !self.session.wallets.is_empty()
            || self.spend.is_some()
    }

    /// Time without input so far, counted across an idle lock.
    pub(crate) fn idle_ms(&self) -> u64 {
        self.last_input
            .map_or(0, |since| self.now_ms.saturating_sub(since))
            + self.idle_carry
    }

    /// When the idle warning comes up and when the lock follows, ms
    /// without input; `None` when the session never locks for idleness.
    pub(crate) fn idle_lock_at(&self) -> Option<(u64, u64)> {
        (self.idle_lock_min > 0).then(|| {
            let lock = u64::from(self.idle_lock_min) * 60_000;
            (lock - IDLE_WARN_MS.min(lock / 2), lock)
        })
    }

    /// Ms without input at which the machine powers off, when it would:
    /// nothing held, nothing in the Outbox, not the online app.
    pub(crate) fn idle_off_at(&self) -> Option<u64> {
        (self.idle_off_min > 0 && !self.online && !self.lockable() && self.outbox.is_empty())
            .then(|| u64::from(self.idle_off_min) * 60_000)
    }

    /// The idle timers (`PLAN.md` §12.3). Returns whether the screen
    /// changed.
    fn idle_tick(&mut self, now_ms: u64) -> bool {
        let since = *self.last_input.get_or_insert(now_ms);
        let idle = now_ms.saturating_sub(since) + self.idle_carry;
        let min = 60_000u64;
        // The warning, with its countdown, before the lock.
        if let Some((warn, lock)) = self.idle_lock_at()
            && idle >= warn
            && idle < lock
            && self.lockable()
            && self.idle_locking.is_none()
        {
            let changed = self.sheet != Some(Sheet::IdleWarn);
            if changed && self.sheet.is_none() {
                self.sheet = Some(Sheet::IdleWarn);
                self.idle_warn_min = idle / min;
            }
            let left = (lock - idle).div_ceil(1000);
            if self.sheet == Some(Sheet::IdleWarn) && self.idle_shown != Some(left) {
                self.idle_shown = Some(left);
                return true;
            }
            return false;
        }
        // The Locked sheet counts down to power-off.
        if self.sheet == Some(Sheet::Locked)
            && let Some(off) = self.idle_off_at()
        {
            let left = off.saturating_sub(idle).div_ceil(1000);
            if self.idle_shown != Some(left) {
                self.idle_shown = Some(left);
                if idle < off {
                    return true;
                }
            }
        }
        if self.idle_lock_min > 0
            && idle >= u64::from(self.idle_lock_min) * min
            && self.lockable()
            && self.idle_locking.is_none()
        {
            self.idle_locking = Some(idle);
            self.lock();
            return true;
        }
        if self.idle_off_min > 0
            && idle >= u64::from(self.idle_off_min) * min
            && !self.online
            && !self.lockable()
        {
            if self.outbox.is_empty() {
                self.commands.push_back(Command::Exit);
            } else if self.sheet.is_none() {
                // Powering off would lose what waits: the lock screen
                // stays, saying what it is.
                self.sheet = Some(Sheet::Locked);
                return true;
            }
        }
        false
    }

    fn sticks_changed(&mut self, sticks: Vec<StickInfo>) {
        let had = !self.sticks.is_empty();
        let ids_before: Vec<String> = self.sticks.iter().map(|s| s.id.clone()).collect();
        self.sticks = sticks;
        // The medium the device started from goes by what it is, not by
        // its volume label.
        let boot = self.medium.boot();
        for s in self.sticks.iter_mut().filter(|s| s.boot) {
            s.label = boot.to_string();
        }
        let ids_now: Vec<String> = self.sticks.iter().map(|s| s.id.clone()).collect();
        if self.visit.stick >= self.sticks.len() {
            self.visit.stick = 0;
        }
        let names: BTreeSet<String> = self
            .sticks
            .get(self.visit.stick)
            .map(|s| s.files.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default();
        self.visit.inn.retain(|n| names.contains(n));
        if self.sticks.is_empty() {
            self.stick_held = false;
            self.not_now = false;
            // The lock Upgrade asked for stays up: it waits on no stick.
            if self.sheet == Some(Sheet::Lock) && !self.upgrade_after_lock {
                self.sheet = None;
            }
            // A passphrase waiting on the stick takes typing now.
            if self.screen == Screen::Unlock
                && self.vaults.focus.is_none()
                && self.vault_files().iter().any(|f| f.open.is_none())
            {
                self.vaults.focus = Some(vaults::Focus::Passphrase);
            }
            if self.screen == Screen::Visit {
                // Where the visit's chip said it goes (§4.7).
                let unlock = self
                    .after_visit
                    .is_none()
                    .then(|| self.visit_unlocks())
                    .flatten();
                self.screen = self.after_visit.take().unwrap_or(Screen::Home);
                if self.screen == Screen::Family {
                    self.family_settle();
                }
                if let Some(i) = unlock {
                    self.vault_act(vaults::VaultAction::Open(i));
                }
            }
            if had {
                let loaded = self.load_after_pull();
                let said = if loaded == 0 {
                    format!("{} removed", self.medium.cap())
                } else {
                    format!(
                        "{} removed · {loaded} {} loaded",
                        self.medium.cap(),
                        if loaded == 1 { "key" } else { "keys" }
                    )
                };
                self.toast(&said);
            }
            self.import_stick_gone();
            // What was pressed with the stick in carries on.
            if self.sheet == Some(Sheet::Pull) {
                self.sheet = None;
                if let Some(a) = self.pull.take() {
                    self.act(a);
                }
            }
            return;
        }
        self.import_stick_gone();
        if ids_now == ids_before {
            self.release_held_stick();
            return;
        }
        // A stick arriving while the camera is on is held back, and
        // arrives when the camera is off (`PLAN.md` §5.4): then a clean
        // process visits it and any other asks to lock first. The
        // scanner is never replaced with its camera still on.
        if self.camera_on() {
            self.stick_held = true;
            return;
        }
        self.stick_arrived();
    }

    /// A stick arrived, or one held back is taken now.
    fn stick_arrived(&mut self) {
        // A stick arrived. Only a clean process takes it; any other locks
        // first (`PLAN.md` §5.4). No passphrase field takes typing while
        // it is attached.
        if matches!(
            self.vaults.focus,
            Some(vaults::Focus::Passphrase | vaults::Focus::Phrase(..) | vaults::Focus::Dice)
        ) {
            self.vaults.focus = None;
        }
        self.visit.log.clear();
        self.visit.settings = None;
        // The first look at the boot stick this power-on: what it holds is
        // copied into memory for the import (`boot_import`).
        let first_look = self.stick_settings.is_none();
        let boot = self.sticks.iter().find(|s| s.boot).cloned();
        if !self.clean() {
            self.not_now = false;
            self.upgrade_after_lock = false;
            self.sheet = Some(Sheet::Lock);
        } else if self.upgrading() {
            // The upgrade reads boot partitions; the data partitions the
            // disk process lists stay where they are, unvisited.
        } else {
            self.visit.out = self.visit_default_out();
            self.visit.inn = self.visit_default_in();
            if self.sheet == Some(Sheet::Import) {
                self.sheet = None;
            }
            // The boot stick alone at its first look leaves Home on
            // screen, with the import over it when it holds anything.
            let boot_only = self.sticks.iter().all(|s| s.boot);
            if !(boot_only && first_look && self.screen == Screen::Home) {
                // A stick arriving in the middle of Create or Restore
                // brings the person back there when it is pulled.
                if matches!(
                    self.screen,
                    Screen::Create | Screen::Restore | Screen::Family
                ) {
                    self.after_visit = Some(self.screen);
                }
                self.screen = Screen::Visit;
            }
            if first_look
                && self.import.is_none()
                && let Some(b) = boot.as_ref()
                && b.files.iter().any(|(n, _)| !stick_settings::is_file(n))
            {
                self.import_start(b);
            }
            // The boot stick's settings, once a session (`stick_settings`).
            if self.stick_settings.is_none()
                && let Some(boot) = self.sticks.iter().find(|s| s.boot)
            {
                match boot.files.iter().find(|(n, _)| stick_settings::is_file(n)) {
                    Some((n, _)) => self.storage_out.push_back(StorageCommand::Read {
                        stick: boot.id.clone(),
                        name: n.clone(),
                    }),
                    None => self.settings_on_stick_known(),
                }
            }
        }
    }

    fn save_boxes(&mut self) {
        let pack = |v: &Vec<Item>| {
            v.iter()
                .map(|i| (i.name.clone(), i.bytes.clone()))
                .collect()
        };
        let kept = self.kept();
        self.storage_out.push_back(StorageCommand::SaveBoxes {
            inbox: pack(&self.inbox),
            outbox: pack(&self.outbox),
            kept,
        });
    }

    /// A file, or files, a flow made and put For the stick: "{what}
    /// waits for the stick".
    fn toast_out(&mut self, what: &str) {
        let text = format!("{what} waits {}", self.medium.for_the());
        self.toast(&text);
    }

    fn toast(&mut self, text: &str) {
        self.toast = Some((text.to_string(), self.now_ms + 3000));
        self.toast_at = None;
    }

    /// How much of the toast shows, 0 to 1, and how far below its place
    /// it is, in units: it rises in, and fades as its time runs out.
    pub(crate) fn toast_shown(&self) -> (f32, f32) {
        let Some((_, until)) = &self.toast else {
            return (0.0, 0.0);
        };
        let rise = motion::ease_out(motion::progress(
            self.toast_at,
            self.now_ms,
            motion::TOAST_MS,
        ));
        let left = until.saturating_sub(self.now_ms) as f32 / motion::TOAST_MS as f32;
        let lift = if self.reduce_motion {
            0.0
        } else {
            10.0 * (1.0 - rise)
        };
        (rise.min(left.min(1.0)), lift)
    }

    /// Whether loading a key is allowed now: only once the self-test has
    /// passed, and never with a stick attached.
    pub fn may_load_keys(&self) -> bool {
        self.selftest_passed() && self.sticks.is_empty()
    }

    /// What `action` does, named for the sheet that asks for the stick to
    /// be pulled first, when it loads a key or turns the camera on (a
    /// frame may picture a seed); `None` for anything a stick may stay in
    /// for.
    pub fn pull_what(&self, action: Action) -> Option<&'static str> {
        use catalog::Go;
        Some(match action {
            Action::Scan | Action::BScan | Action::Vault(vaults::VaultAction::ScanEntry) => "scan",
            Action::Entry(_) | Action::ScanSeed | Action::ScanPart | Action::LoadKey(_) => {
                "add a key"
            }
            Action::KeyGen(_) | Action::KeyGenSlip39 => "make a key",
            Action::PotentialOpen(_) => "load the wallet",
            Action::BackupOpen(_) => "open the backup",
            // Unlocking for another flow: the Pull sheet asks first, and
            // Unlock opens once the stick is out.
            Action::Vault(vaults::VaultAction::OpenFrom(i, _))
                if self.vault_files().get(i).is_some_and(|f| f.open.is_none()) =>
            {
                "unlock"
            }
            Action::Vault(vaults::VaultAction::ListFrom(_)) => "unlock",
            Action::Catalog(i) => match catalog::TILES.get(usize::from(i))?.go {
                Go::VaultCategory(_)
                    if self.vaults.open.is_empty() && !self.vault_files().is_empty() =>
                {
                    "unlock"
                }
                Go::NewKey | Go::NewShares => "make a key",
                Go::AddKey(_) | Go::SeedQr => "add a key",
                // Sign and Decode with nothing to open scan: the Scan
                // they press waits instead.
                Go::CheckAddress | Go::Scan => "scan",
                _ => return None,
            },
            _ => return None,
        })
    }

    /// What the start-up self-test found, once it has run: how many
    /// checks passed, or the name of the first that failed.
    pub fn selftest(&self) -> Option<osk_selftest::Outcome> {
        self.selftest
    }

    /// The self-test has run and every check passed.
    pub fn selftest_passed(&self) -> bool {
        matches!(self.selftest, Some(Ok(_)))
    }

    /// The self-test has run and a check failed: the app is blocked.
    pub fn selftest_failed(&self) -> bool {
        matches!(self.selftest, Some(Err(_)))
    }

    /// Runs `checks` as the self-test, in place of
    /// [`osk_selftest::CHECKS`]: for a test that a failed check blocks the
    /// app.
    #[doc(hidden)]
    pub fn run_selftest_with(&mut self, checks: &[osk_selftest::Check]) {
        self.selftest = Some(osk_selftest::run_checks(checks));
        if self.selftest_failed() {
            self.sheet = None;
        }
        self.dirty = true;
    }

    /// The toast shown, if one is current.
    pub fn toast_text(&self) -> Option<&str> {
        self.toast.as_ref().map(|(t, _)| t.as_str())
    }

    /// A public or sealed file to the Outbox. A secret never comes this
    /// way: it goes through [`Faraday::offer_secret`], which asks first.
    fn put_outbox(&mut self, name: &str, bytes: Vec<u8>) {
        let item = Item::new(name, bytes);
        if item.exposure() == secrets::Exposure::Secret {
            debug_assert!(false, "a secret reached put_outbox: {name}");
            return;
        }
        self.outbox.retain(|i| i.name != name);
        self.outbox.push(item);
        self.save_boxes();
    }

    /// A write's answer onto the receipt: the first answer of a write
    /// replaces the last receipt with a new one (`docs/SIMPLIFY.md` §4.3).
    fn receipt_add(&mut self, name: &str, failure: Option<String>) {
        let now = self.clock_now();
        let Some((label, sent, started)) = self.visit.writing.as_mut() else {
            return;
        };
        let Some(k) = sent.iter().position(|(n, _)| n == name) else {
            return;
        };
        let (name, hash) = sent.remove(k);
        if !*started {
            *started = true;
            self.receipt = Some(Receipt {
                label: label.clone(),
                time: now,
                files: Vec::new(),
            });
        }
        if sent.is_empty() {
            self.visit.writing = None;
        }
        if let Some(r) = self.receipt.as_mut() {
            r.files.push(ReceiptFile {
                name,
                hash,
                verified: failure.is_none(),
                failure,
            });
        }
    }

    /// What a visit writes unless the person changes it: the Outbox but
    /// an unprotected secret, which is written only when ticked on the
    /// visit itself.
    /// The vault, by place in [`Faraday::vault_files`], a stick visit
    /// unlocks once the stick is pulled: a locked one sealed for it, still
    /// For the stick or written by the last write.
    fn visit_unlocks(&self) -> Option<usize> {
        self.vault_files().iter().position(|f| {
            f.open.is_none()
                && (f.in_outbox || self.receipt.as_ref().is_some_and(|r| r.wrote(&f.name)))
        })
    }

    /// The stick visit's chip (`docs/SIMPLIFY.md` §4.7): where pulling
    /// the stick goes. Back to the flow the stick came in during, Unlock
    /// when a vault was sealed for the visit, else Home.
    pub fn visit_then(&self) -> String {
        match self.after_visit {
            Some(s) => format!("Then: {}", self.back_name(s)),
            None if self.visit_unlocks().is_some() => "Then: Unlock".to_string(),
            None => "Then: Home".to_string(),
        }
    }

    /// Remembers that vault file `name`, these bytes, was read from the
    /// stick labelled `label`.
    pub(crate) fn vault_read_from(&mut self, name: &str, bytes: &[u8], label: &str) {
        self.vault_from.retain(|(n, ..)| n != name);
        self.vault_from
            .push((name.to_string(), sha256_of(bytes), label.to_string()));
    }

    /// What a visit ticks to come in (`docs/SIMPLIFY.md` §4.6): every file
    /// on the stick shown that Faraday reads, but a seed's by its name
    /// (which comes in through its own tick), one over 18 MiB, and one
    /// From the stick already by name and size.
    pub(crate) fn visit_default_in(&self) -> std::collections::BTreeSet<String> {
        let Some(stick) = self.sticks.get(self.visit.stick) else {
            return Default::default();
        };
        stick
            .files
            .iter()
            .filter(|(n, size)| {
                stick_kind(n).is_some()
                    && *size <= READ_MAX
                    && !seed_by_name(n)
                    && !self
                        .inbox
                        .iter()
                        .any(|i| i.name == *n && i.bytes.len() as u64 == *size)
            })
            .map(|(n, _)| n.clone())
            .collect()
    }

    /// Every Outbox file a visit may write unasked: all but an
    /// unprotected secret.
    fn visit_all_out(&self) -> std::collections::BTreeSet<String> {
        self.outbox
            .iter()
            .filter(|i| i.exposure() != secrets::Exposure::Secret)
            .map(|i| i.name.clone())
            .collect()
    }

    /// [`Faraday::visit_all_out`], but with more than one vault file
    /// waiting, one vault for this stick (`docs/NEW-WALLET.md` §5.3): the
    /// first the last write did not write. The rest wait for the next
    /// stick.
    fn visit_default_out(&self) -> std::collections::BTreeSet<String> {
        let mut out = self.visit_all_out();
        let vaults: Vec<&Item> = self
            .outbox
            .iter()
            .filter(|i| i.kind == FileKind::Vault)
            .collect();
        if vaults.len() > 1 {
            let written = |i: &Item| {
                self.receipt
                    .as_ref()
                    .is_some_and(|r| r.holds(&i.name, &i.bytes))
            };
            let one = vaults
                .iter()
                .find(|i| !written(i))
                .or(vaults.first())
                .map(|i| i.name.clone());
            for i in &vaults {
                if Some(&i.name) != one.as_ref() {
                    out.remove(&i.name);
                }
            }
        }
        out
    }

    fn lock(&mut self) {
        self.vault_seal_all();
        // What the boot stick brought and was not imported is not kept:
        // its bytes are wiped as it drops.
        self.import = None;
        // Nothing secret copied in outlives the process that read it
        // (`PLAN.md` §5.3): the next process is clean, and these would be
        // in its memory and in `/run/faraday`. They are still on the stick
        // they came from.
        // A picture goes as what its codes hold, and as Other when none
        // read.
        self.inbox
            .retain(|i| !i.holds().may_be_secret() && !i.secret);
        self.session.wipe();
        self.spend = None;
        self.backup = None;
        self.message = None;
        self.checking = None;
        self.create = None;
        self.restore = None;
        self.keygen = None;
        self.wordlist = None;
        self.bip85 = None;
        self.secret_out = None;
        self.entry = EntryState::default();
        self.save_boxes();
        self.restart = true;
        self.commands.push_back(Command::Exit);
    }

    fn start_spend(&mut self, index: usize) {
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        // A carry file is the PSBT and the secret nonce of the share that
        // signs next.
        let (psbt, carry) = if item.kind == FileKind::Carry {
            match osk_psbt::threshold::Carry::parse(&item.bytes) {
                Ok(c) => match osk_psbt::Psbt::parse_bytes(&c.psbt) {
                    Ok(p) => (p, Some(c.section)),
                    Err(_) => {
                        self.toast("The carry file holds no PSBT");
                        return;
                    }
                },
                Err(e) => {
                    self.toast(e.reason());
                    return;
                }
            }
        } else {
            let Some(psbt) = wallet::read_psbt(&item.bytes) else {
                self.toast("Not a PSBT");
                return;
            };
            // A round an open vault keeps for this transaction.
            let carry = self.vault_round(&psbt);
            (psbt, carry)
        };
        let name = item.name.clone();
        if let Err(e) = self.session.follow_psbt(&psbt) {
            self.toast(&e.text());
            return;
        }
        let spend = Spend::new(psbt, &name);
        let inspection = self.session.inspect(&spend.psbt);
        let signers = spend.signers(&self.session).iter().map(|f| f.0).collect();
        let mut state = SpendState {
            spend,
            // Computed in `refresh_spend`, once Check's done state is
            // known: open at Check, or past it when this wallet is
            // already checked this power-on.
            open: None,
            done: [false; 10],
            table: false,
            error: None,
            inspection,
            signers,
            out_signed: false,
            out_tx: false,
            show_hex: false,
            nonce_check: Default::default(),
            scroll: 0.0,
            follow: false,
            wallet: None,
            steps: Vec::new(),
            needed: 1,
            complete: false,
            musig: None,
            others: Vec::new(),
            carry,
            carry_out: None,
            round_spent: false,
            opened: false,
        };
        state.done[0] = true;
        self.spend = Some(state);
        self.refresh_spend();
        self.screen = Screen::Spend;
    }

    fn refresh_spend(&mut self) {
        if let Some(s) = self.spend.as_mut() {
            s.inspection = self.session.inspect_with(&s.spend.psbt, s.carry.as_ref());
            s.signers = s.spend.signers(&self.session).iter().map(|f| f.0).collect();
            s.wallet = self.session.wallet_for(&s.spend.psbt);
            let wallet = s.wallet.and_then(|w| self.session.wallets.get(w));
            let kind = wallet.map(|w| wallet::Kind::of(&w.policy));
            s.needed = wallet.map(wallet::needed).unwrap_or(1);
            // With no wallet loaded the signers come from the PSBT alone,
            // and a second device may still be needed.
            let kind = kind.unwrap_or(if s.inspection.participating_keys.len() > 1 {
                wallet::Kind::Multi(osk_bip::policy::Wrapper::Wsh)
            } else {
                wallet::Kind::Single(osk_bip::keys::ScriptType::NativeSegwit)
            });
            s.steps = kind.steps();
            s.complete = s.spend.finished.is_some() || s.spend.finish().is_ok();
            // Check is done already, this power-on, when this wallet was
            // checked before (`docs/WALLETS.md` §4): the flow opens on
            // the next card without a default instead.
            if let Some(wallet) = wallet {
                let checksum = wallet.policy.checksum();
                if self.checked_wallets.contains(&checksum) {
                    s.done[wallet::step::CHECK as usize] = true;
                }
            }
            let stale = s.open.is_some_and(|open| !s.steps.contains(&open));
            if stale || !s.opened {
                s.open = s.steps.iter().copied().find(|&n| !s.done[n as usize]);
                s.opened = true;
            }
        }
        self.spend_files_out();
    }

    /// Finish makes both results For the stick the moment the
    /// transaction is finished: the signed PSBT and the finished
    /// transaction. One the person removed is not made again unless they
    /// ask.
    fn spend_files_out(&mut self) {
        let Some(s) = self.spend.as_mut() else {
            return;
        };
        if s.spend.finished.is_none() {
            return;
        }
        let stem = result_stem(&s.spend.source);
        let mut made: Vec<(String, Vec<u8>)> = Vec::new();
        if !s.out_signed {
            s.out_signed = true;
            made.push((format!("{stem}-signed.psbt"), s.spend.psbt.to_bytes()));
        }
        if !s.out_tx
            && let Some(hex) = s.spend.finished_hex()
        {
            s.out_tx = true;
            made.push((format!("{stem}-final.txn"), hex.into_bytes()));
        }
        for (name, bytes) in made {
            self.put_outbox(&name, bytes);
        }
    }

    /// The PSBT with what this pass added, For the stick for the other
    /// signers while more are needed. A threshold spend's part goes with
    /// its carry, through the secret sheet, instead.
    fn part_out(&mut self) {
        let Some(s) = self.spend.as_ref() else {
            return;
        };
        let threshold = s
            .inspection
            .inputs
            .first()
            .is_some_and(|i| i.threshold.is_some());
        if s.complete || threshold || s.carry_out.is_some() {
            return;
        }
        let name = format!("{}-part.psbt", result_stem(&s.spend.source));
        let bytes = s.spend.psbt.to_bytes();
        self.put_outbox(&name, bytes);
    }

    /// The session strip's stage, computed, never stored: where the
    /// person is in the lock cycle now. A stick attached while working
    /// still asks to bring it in, because the lock sheet is what asks to
    /// lock it; `Write out` is files For the stick, or a vault changed
    /// since it was written (§3.5), with no stick attached.
    pub fn session_stage(&self) -> &'static str {
        if !self.sticks.is_empty() || self.import.is_some() {
            "Bring in"
        } else if !self.outbox.is_empty() || self.vault_changed().is_some() {
            "Write out"
        } else if self.holds_secret() || !self.session.wallets.is_empty() {
            "Work"
        } else {
            "Open"
        }
    }

    /// The Inbox PSBT to offer first: the one with the fewest signatures,
    /// so a cosigner's signed copy never stands in for the transaction.
    pub fn lead_psbt(&self) -> Option<usize> {
        self.inbox
            .iter()
            .enumerate()
            .filter(|(_, it)| it.kind == FileKind::Psbt)
            .filter_map(|(i, it)| {
                let p = wallet::read_psbt(&it.bytes)?;
                let sigs: usize = p
                    .inner()
                    .inputs
                    .iter()
                    .map(|inp| inp.partial_sigs.len())
                    .sum();
                Some((sigs, i))
            })
            .min()
            .map(|(_, i)| i)
    }

    /// Signatures the open spend's wallet needs.
    pub fn spend_needed(&self) -> usize {
        self.spend.as_ref().map(|s| s.needed).unwrap_or(1)
    }

    fn act(&mut self, action: Action) {
        // A build whose vectors do not reproduce does nothing but exit.
        if self.selftest_failed() {
            if action == Action::PowerOff {
                self.commands.push_back(Command::Exit);
            }
            return;
        }
        // A chart's sheet leads into a flow: whatever its rows press, the
        // sheet goes first.
        if self.sheet == Some(Sheet::Chart) && !matches!(action, Action::Chart(_)) {
            self.sheet = None;
            self.chart = None;
        }
        // Keys load only with no stick attached: what loads one waits,
        // under a sheet, for the stick to be pulled.
        if self.selftest_passed() && !self.sticks.is_empty() && self.pull_what(action).is_some() {
            self.pull = Some(action);
            self.sheet = Some(Sheet::Pull);
            return;
        }
        match action {
            Action::SelfTestRun => {
                self.selftest = Some(osk_selftest::run());
                if self.selftest_failed() {
                    self.sheet = None;
                }
            }
            Action::Vault(v) => {
                self.vault_act(v);
                // What changed in an open vault is what a lock remembers.
                self.vault_summaries_refresh();
            }
            Action::Import(a) => self.import_act(a),
            Action::Family(f) => self.family_act(f),
            Action::Vanity(v) => self.vanity_act(v),
            Action::Learn => self.learn_open(),
            Action::LearnShares => self.learn_shares(),
            Action::LearnSpend => {
                self.sheet = None;
                self.screen = Screen::Family;
            }
            Action::Osk(p) => self.osk_press(p),
            Action::ScanCamera(i) => {
                if let Some((id, _)) = self.cameras.get(usize::from(i)) {
                    self.camera = Some(id.clone());
                    self.camera_change = Some(id.clone());
                    if let Some(s) = self.scan.as_mut() {
                        s.seen = None;
                        s.too_fine = 0;
                    }
                }
            }
            Action::Bip85
            | Action::PStep(_)
            | Action::PKey(_)
            | Action::PApp(_)
            | Action::PLength(_)
            | Action::PIndex(_)
            | Action::PNext
            | Action::PShow
            | Action::PVault
            | Action::PLoad
            | Action::POut => self.bip85_act(action),
            Action::Silent
            | Action::SStep(_)
            | Action::SKey(_)
            | Action::SLabel(_)
            | Action::SQr(_)
            | Action::SRecord
            | Action::SScanVault
            | Action::SScanOut
            | Action::SNext
            | Action::SAddWallet
            | Action::SWallet(_)
            | Action::SCheck(_) => self.silent_act(action),
            Action::Explore
            | Action::XKey(_)
            | Action::XPreset(_)
            | Action::XScript(_)
            | Action::XIndex(_) => self.explore_act(action),
            Action::Lightning
            | Action::LKey(_)
            | Action::LAezeed
            | Action::LPassphrase
            | Action::LResolve
            | Action::LShow
            | Action::LVault
            | Action::LOut => self.lightning_act(action),
            Action::Tools | Action::TTool(_) | Action::TMode(_) | Action::TClear => {
                self.tools_act(action)
            }
            Action::ExploreKey(fp) => {
                self.explore_act(Action::Explore);
                self.explore_act(Action::XKey(fp));
            }
            Action::SecretVault => self.secret_to_vault(),
            Action::SecretAck => self.secret_ack = !self.secret_ack,
            Action::SecretUnprotected => self.secret_unprotected(),
            Action::SecretForm(i) => self.secret_form(usize::from(i)),
            Action::LearnPage(i) => self.learn_page(i),
            Action::WordList(a) => self.wordlist_act(a),
            Action::KeyGen(_)
            | Action::KStep(_)
            | Action::KWords(_)
            | Action::KWay(_)
            | Action::KGroup(_)
            | Action::KMix(_)
            | Action::KRoll(_)
            | Action::KFlip(_)
            | Action::KDie(_)
            | Action::KByDie(_)
            | Action::KTyping(_)
            | Action::KRank(_)
            | Action::KSuit(_)
            | Action::KHex(_)
            | Action::KFrame
            | Action::KUndo
            | Action::KClear
            | Action::KNext
            | Action::KShow
            | Action::KAdd
            | Action::KLock
            | Action::KPassField(_)
            | Action::KQuiz(_)
            | Action::KQuizRetry
            | Action::KSkip
            | Action::KAgain
            | Action::KForm(_)
            | Action::KSlipWords(_)
            | Action::KSlipM(_)
            | Action::KSlipN(_)
            | Action::KShare(_)
            | Action::KeyGenSlip39 => self.keygen_act(action),
            Action::Nav(Screen::Transfer) => self.transfer_open(),
            Action::TransferSend(i) => self.transfer_pick(i),
            Action::TransferReceive => self.transfer_receive(),
            Action::TransferOpenFolder => self.transfer_open_folder(),
            Action::UpgradeOpen => self.upgrade_open(),
            Action::UpgradePick(i) => self.upgrade_pick(i),
            Action::UpgradeWrite => self.upgrade_write(),
            Action::UpgradeAgain => self.upgrade_again(),
            Action::BackupsOf(i) => {
                let sum = self.session.wallets.get(i).map(|w| w.policy.checksum());
                let at = self
                    .backups()
                    .iter()
                    .position(|e| Some(&e.sum) == sum.as_ref())
                    .unwrap_or(0);
                self.act(Action::Nav(Screen::Backups));
                self.backups_at = at;
                if !self.compact {
                    self.list_offset = crate::backups_screen::offset_of(self, at);
                }
            }
            Action::BackupsPage(i) => {
                self.backups_at = i;
                self.list_offset = 0.0;
            }
            Action::Nav(s) => {
                self.chart_focus = None;
                if s == Screen::Backups {
                    self.backups_at = 0;
                }
                self.screen = s;
                self.osk_leave();
                self.list_offset = 0.0;
                self.card_offset = 0.0;
                self.glance = false;
                self.renaming = None;
                if s == Screen::Visit {
                    self.visit.out = self.visit_default_out();
                    self.visit.inn = self.visit_default_in();
                    self.visit.from_inbox.clear();
                    self.visit.out_offset = 0.0;
                }
            }
            Action::Entry(wanted) => {
                if self.may_load_keys() {
                    let back = matches!(
                        self.screen,
                        Screen::Restore | Screen::Family | Screen::Spend
                    )
                    .then_some(self.screen);
                    self.entry = EntryState::default();
                    self.entry.wanted = wanted;
                    self.entry.back = back;
                    // A small panel has no keyboard of its own: every
                    // list's words go on OpenSigner's word keyboard.
                    if self.compact {
                        self.entry.keys = Some(forms::word_typer(self.entry.language()));
                    }
                    self.screen = Screen::Entry;
                }
            }
            Action::EntryClear => {
                self.entry.typed.clear();
                if self.entry.keys.is_some() {
                    self.entry.keys = Some(forms::word_typer(self.entry.language()));
                }
                self.entry.label.clear();
                self.entry.error = None;
            }
            Action::EntryAdd => self.entry_add(),
            Action::EntryLanguages => self.entry.languages = !self.entry.languages,
            Action::EntryLanguage(i) => {
                if let Some(&lang) = forms::LANGUAGES.get(usize::from(i)) {
                    self.entry.language_at = i;
                    self.entry.error = None;
                    self.entry.shift = false;
                    // A list with a keyboard of its own starts its words
                    // over; one typed in Latin letters keeps them.
                    self.entry.keys =
                        (forms::on_screen(lang) || self.compact).then(|| forms::word_typer(lang));
                    if self.compact {
                        self.entry.languages = false;
                    }
                }
            }
            Action::EntryKey(c) => {
                let lang = self.entry.language();
                if let Some(w) = self.entry.keys.as_mut() {
                    w.type_char(c);
                    self.entry.shift = false;
                    self.entry.error = None;
                    if forms::word_whole(w, lang) {
                        forms::take_typed(w, lang);
                    }
                }
            }
            Action::EntryKeyBack => {
                if let Some(w) = self.entry.keys.as_mut() {
                    w.backspace();
                    self.entry.error = None;
                }
            }
            Action::EntryShift => self.entry.shift = !self.entry.shift,
            Action::EntryCandidate(n) => {
                if let Some(w) = self.entry.keys.as_mut() {
                    w.commit_candidate(usize::from(n));
                    self.entry.error = None;
                }
            }
            Action::EntryForm(i) => {
                if let Some(&f) = forms::Form::ALL.get(usize::from(i)) {
                    let (wanted, back) = (self.entry.wanted, self.entry.back);
                    self.entry = EntryState::default();
                    self.entry.wanted = wanted;
                    self.entry.back = back;
                    self.entry.form = f;
                    // Chosen from the chips, or opened by a Tools tile
                    // for this form: the chips stay in view.
                    self.entry.other_forms = true;
                }
            }
            Action::EntryOtherForms => self.entry.other_forms = true,
            Action::EntryPart => self.form_add_part(),
            Action::EntryRecover => self.form_recover(),
            Action::PickWallet(i) => {
                self.wallet = i;
                self.renaming = None;
                self.card_offset = 0.0;
            }
            Action::PickKey(fp) => self.loose_pick = Some(fp),
            Action::RemoveWallet(i) => {
                if i < self.session.wallets.len() {
                    self.session.wallets.remove(i);
                    self.wallet = 0;
                }
            }
            Action::RemoveKeys => {
                self.session.keys.clear();
                self.spend = None;
            }
            Action::StartSpend(i) => self.start_spend(i),
            Action::Step(n) => {
                if let Some(s) = self.spend.as_mut() {
                    // On the Spend tab a step opened from another card
                    // stays open.
                    let from_page = self.screen == Screen::Family
                        && self.family.open != Some(family::Open::Spend);
                    s.open = if s.open == Some(n) && !from_page {
                        None
                    } else {
                        Some(n)
                    };
                    s.error = None;
                    s.follow = true;
                }
                if self.screen == Screen::Family {
                    self.family_step_moved(false);
                }
            }
            Action::StepNext(n) => {
                if let Some(s) = self.spend.as_mut() {
                    s.done[n as usize] = true;
                    s.open = s.steps.iter().copied().find(|&i| !s.done[i as usize]);
                    s.error = None;
                    s.follow = true;
                }
                // Check's own Continue: this wallet is checked this
                // power-on (`docs/WALLETS.md` §4), kept across a lock.
                if n == wallet::step::CHECK
                    && let Some(w) = self.spend.as_ref().and_then(|s| s.wallet)
                    && let Some(wallet) = self.session.wallets.get(w)
                {
                    let checksum = wallet.policy.checksum();
                    self.mark_checked(&checksum);
                }
                if self.screen == Screen::Family {
                    self.family_step_moved(true);
                }
            }
            Action::ToggleTable => {
                if let Some(s) = self.spend.as_mut() {
                    s.table = !s.table;
                }
            }
            Action::SignHere => self.sign_here(),
            Action::Collect(i) => self.collect(i),
            Action::SignedToOutbox => {
                if let Some(s) = self.spend.as_mut() {
                    let name = format!("{}-signed.psbt", result_stem(&s.spend.source));
                    let bytes = s.spend.psbt.to_bytes();
                    s.out_signed = true;
                    self.put_outbox(&name, bytes);
                }
            }
            Action::TxToOutbox => {
                if let Some(s) = self.spend.as_mut()
                    && let Some(hex) = s.spend.finished_hex()
                {
                    let name = format!("{}-final.txn", result_stem(&s.spend.source));
                    s.out_tx = true;
                    self.put_outbox(&name, hex.into_bytes());
                }
            }
            Action::Primary => self.primary(),
            Action::LoadWallet(i) => self.load_wallet(i),
            Action::InboxRemove(i) => {
                if i < self.inbox.len() {
                    self.inbox.remove(i);
                    self.save_boxes();
                }
            }
            Action::OutboxRemove(i) => {
                if i < self.outbox.len() {
                    self.outbox.remove(i);
                    self.save_boxes();
                }
            }
            Action::VisitStick(i) => {
                self.visit.stick = i;
                self.visit.inn = self.visit_default_in();
                self.visit.settings = None;
            }
            Action::VisitSettings => {
                self.visit.settings = Some(!self.visit_settings_on());
            }
            Action::VisitOut(i) => {
                if let Some(item) = self.outbox.get(i) {
                    let n = item.name.clone();
                    if !self.visit.out.remove(&n) {
                        self.visit.out.insert(n);
                    }
                }
            }
            Action::VisitIn(i) => {
                if let Some(stick) = self.sticks.get(self.visit.stick)
                    && let Some((n, size)) = stick.files.get(i)
                    && *size <= READ_MAX
                {
                    let n = n.clone();
                    if !self.visit.inn.remove(&n) {
                        self.visit.inn.insert(n);
                    }
                }
            }
            // Unselect all: everything comes in by default (§4.6).
            Action::VisitInAll => self.visit.inn.clear(),
            Action::VisitOutAll => {
                // Every row but an unprotected secret, which is ticked one
                // at a time (FLOWS.md decision 6); from the Inbox, every
                // public or sealed file, and none that may be a secret.
                let all = self.visit_all_out();
                let inbox = self.visit_inbox_plain();
                let every = self.visit_settings_on()
                    && all.is_subset(&self.visit.out)
                    && inbox.is_subset(&self.visit.from_inbox);
                if every {
                    self.visit.out.clear();
                    for n in &inbox {
                        self.visit.from_inbox.remove(n);
                    }
                } else {
                    self.visit.out.extend(all);
                    self.visit.from_inbox.extend(inbox);
                }
                self.visit.settings = Some(!every);
            }
            Action::VisitInbox(i) => self.visit_inbox(i),
            Action::VisitBar(_) => {}
            Action::VisitWrite => {
                if let Some(stick) = self.sticks.get(self.visit.stick) {
                    let id = stick.id.clone();
                    let label = stick.label.clone();
                    self.visit.log.clear();
                    let before = self.storage_out.len();
                    if self.visit_settings_on() {
                        self.storage_out.push_back(StorageCommand::Write {
                            stick: id.clone(),
                            name: stick_settings::FILE.to_string(),
                            bytes: self.settings_file_text().into_bytes(),
                        });
                    }
                    for item in &self.outbox {
                        if self.visit.out.contains(&item.name) {
                            self.storage_out.push_back(StorageCommand::Write {
                                stick: id.clone(),
                                name: item.name.clone(),
                                bytes: item.bytes.clone(),
                            });
                        }
                    }
                    // A copy from another stick: its bytes as they came,
                    // a picture's too. Only those the visit lists.
                    let listed = self.visit_inbox_rows();
                    for &k in &listed {
                        let item = &self.inbox[k];
                        if self.visit.from_inbox.contains(&item.name) {
                            self.visit.writing_inbox.insert(item.name.clone());
                            self.storage_out.push_back(StorageCommand::Write {
                                stick: id.clone(),
                                name: item.name.clone(),
                                bytes: item.bytes.clone(),
                            });
                        }
                    }
                    // What this write sends, for its receipt.
                    let sent: Vec<(String, [u8; 32])> = self
                        .storage_out
                        .iter()
                        .skip(before)
                        .filter_map(|c| match c {
                            StorageCommand::Write { name, bytes, .. } => {
                                Some((name.clone(), sha256_of(bytes)))
                            }
                            _ => None,
                        })
                        .collect();
                    if !sent.is_empty() {
                        self.visit.writing = Some((label, sent, false));
                    }
                }
            }
            Action::VisitCopy => self.visit_copy(),
            Action::VisitCopyAndLoad => {
                self.visit.load_after.extend(self.visit.inn.iter().cloned());
                self.visit_copy();
                // Straight to the Inbox, where they arrive; the stick's
                // pull loads them from there.
                self.act(Action::Nav(Screen::Files));
            }
            Action::LoadKey(i) => {
                if self.load_key(i) {
                    self.inbox_load_named();
                }
            }
            Action::Lock => self.lock(),
            Action::EntryPassphrase => self.entry.on_passphrase = !self.entry.on_passphrase,
            Action::LockAsk => {
                let (keys, wallets) = self.unsaved();
                let sealed = self.vaults.open.iter().any(|v| v.changes > 0);
                if !self.vaults.open.is_empty()
                    && (sealed || !keys.is_empty() || !wallets.is_empty())
                {
                    self.sheet = Some(Sheet::LockAsk);
                } else {
                    self.lock();
                }
            }
            Action::NotNow => {
                self.sheet = None;
                // Asked for by Upgrade, with no stick in: nothing waits.
                self.not_now = !std::mem::take(&mut self.upgrade_after_lock);
            }
            Action::PowerAsk => {
                if self.outbox.is_empty() {
                    self.commands.push_back(Command::Exit);
                } else {
                    self.sheet = Some(Sheet::Power);
                }
            }
            Action::PowerOff => {
                self.sheet = None;
                self.commands.push_back(Command::Exit);
            }
            Action::InputUse(id) => self.input_believe(id),
            Action::InputIgnore(id) => self.input_ignore(id),
            Action::IdleLock(m) => {
                self.idle_lock_min = m;
                self.save_boxes();
            }
            Action::IdleOff(m) => {
                self.idle_off_min = m;
                self.save_boxes();
            }
            Action::PublicOut(i, what) => self.public_out(i, what),
            Action::PublicRemove(i, what) => self.public_remove(i, what),
            Action::WalletKeyOut(i, k) => self.wallet_key_out(i, k),
            Action::WalletKeyBsms(i, k) => self.wallet_key_bsms(i, k),
            Action::RFromVault(v, r) => self.restore_from_vault(v, r),
            Action::VisitFrom(back) => {
                if !self.sticks.is_empty() && !self.holds_secret() {
                    self.after_visit = Some(back);
                    self.act(Action::Nav(Screen::Visit));
                }
            }
            Action::Network(n)
                if self.online
                    && n.is_mainnet()
                    && !self.airgap_warned
                    && !self.session.network().is_mainnet()
                    && self.session.wallets.is_empty() =>
            {
                self.mainnet_asked = Some(n);
                self.sheet = Some(Sheet::NotAirgapped);
            }
            Action::AirgapUnderstood => {
                self.airgap_warned = true;
                self.sheet = None;
                if let Some(n) = self.mainnet_asked.take() {
                    self.act(Action::Network(n));
                }
            }
            Action::Network(n) => match self.session.set_network(n) {
                Ok(()) => {
                    self.sheet = None;
                    self.refresh_spend();
                }
                Err(e) => self.toast(&e.text()),
            },
            Action::Cancel => {
                self.pull = None;
                self.mainnet_asked = None;
                self.secret_cancel();
                self.wordlist = None;
                self.potential = None;
                // A transfer in parts survives closing the camera: the
                // parts read so far wait for the next scan.
                if let Some(s) = self.scan.as_mut()
                    && s.in_parts()
                    && s.purpose != ScanPurpose::Transfer
                {
                    self.scan_parts = Some((
                        std::mem::take(&mut s.decoder),
                        std::mem::take(&mut s.assembler),
                    ));
                }
                if self.scan.take().is_some() {
                    self.commands.push_back(Command::CameraOff);
                }
                self.sheet = None;
                self.qr = None;
                self.chart = None;
                self.chart_work.secret.clear();
                self.chart_work.ask = None;
                if let Some(b) = self.backup.as_mut() {
                    b.pick = false;
                }
                // A device still waiting comes back up.
                self.input_sheet();
            }
            Action::About(k) => {
                let this = (self.screen, k);
                self.about_open = (self.about_open != Some(this)).then_some(this);
            }
            Action::Guided(on) => {
                if on != self.guided {
                    // The switch's pill slides from where it shows now.
                    let from = self.guided_shown();
                    self.guided_moving = (!self.reduce_motion).then_some((from, None));
                }
                self.guided = on;
                self.save_boxes();
            }
            Action::SealAmounts(on) => {
                self.seal_amounts = on;
                self.save_boxes();
            }
            Action::OpenWallet(i) => {
                if i < self.session.wallets.len() {
                    self.wallet = i;
                    self.screen = Screen::Wallets;
                    self.card_offset = 0.0;
                    self.glance = false;
                    self.chart_focus = None;
                }
            }
            Action::Chart(c) => self.chart_act(c),
            Action::Glance(open) => {
                self.glance = open && self.screen == Screen::Wallets;
                self.list_offset = 0.0;
            }
            Action::BackupChecklist(i) => {
                // The plan the chart shows, at its first item not done;
                // nothing is made For the stick by opening it.
                if self.chart_checklist(i, false) {
                    let first = self
                        .backup_items()
                        .into_iter()
                        .find(|&n| bstep::item(n).is_some_and(|it| !self.backup_item_done(it)));
                    self.backup_item_open(first);
                    self.screen = Screen::Backup;
                }
            }
            Action::Backup(i) => {
                if i < self.session.wallets.len() {
                    let keys = self.backup_keys(i);
                    let first_key = keys.first().copied().unwrap_or(0);
                    let shape = self.plan_shape(i);
                    // The plan this wallet's last backup saved into an
                    // open vault, else the presets and the wallet's
                    // defaults.
                    let (answers, names, q) = match self.plan_load(i, &shape) {
                        Some((a, names)) => (a, names, None),
                        None => (
                            plan::Answers::defaults(&shape),
                            Vec::new(),
                            Some(qstep::PRESET),
                        ),
                    };
                    // A copy checked this power-on stays checked.
                    let checked = self.backups_checked(i);
                    self.backup = Some(BackupState {
                        wallet: i,
                        q,
                        answers,
                        names,
                        checked,
                        key: first_key,
                        words: 24,
                        ..BackupState::default()
                    });
                    self.screen = Screen::Backup;
                }
            }
            Action::BQ(n) => {
                if let Some(b) = self.backup.as_mut() {
                    b.q = if b.q == Some(n) { None } else { Some(n) };
                    b.scroll.follow = true;
                    b.naming = None;
                }
            }
            Action::BQNext(n) => {
                // Every vault's stick has a place before the plan goes on
                // (`docs/NEW-WALLET.md` §14.2).
                if n == qstep::PLACES && self.backup_stickless().is_some() {
                    return;
                }
                let qs = self.backup_questions();
                if let Some(b) = self.backup.as_mut() {
                    b.q = qs.iter().skip_while(|&&q| q != n).nth(1).copied();
                    b.scroll.follow = true;
                    b.naming = None;
                }
            }
            Action::BPreset(k) => {
                let Some(w) = self.backup.as_ref().map(|b| b.wallet) else {
                    return;
                };
                let shape = self.plan_shape(w);
                if let Some(&preset) = plan::Preset::ALL.get(usize::from(k)) {
                    let answers = plan::Answers::preset(&shape, preset);
                    if let Some(b) = self.backup.as_mut() {
                        b.answers = answers;
                    }
                    self.act(Action::BQNext(qstep::PRESET));
                }
            }
            Action::BAnswer(list, row) => {
                let Some(w) = self.backup.as_ref().map(|b| b.wallet) else {
                    return;
                };
                let shape = self.plan_shape(w);
                if let (Some(q), Some(b)) = (qrow::question(list), self.backup.as_mut()) {
                    b.answers.toggle(&shape, q, usize::from(row));
                    // A question that no longer applies closes.
                    b.naming = b.naming.filter(|&p| p < b.answers.places);
                }
            }
            Action::BName(p) => {
                let open = self.vaults.open.get(self.vaults.current).is_some();
                if let Some(b) = self.backup.as_mut()
                    && open
                {
                    let p = usize::from(p);
                    if b.names.len() <= p {
                        b.names.resize(p + 1, String::new());
                    }
                    b.naming = if b.naming == Some(p) { None } else { Some(p) };
                }
            }
            Action::BChecklist => {
                // A plan that keeps no copy of a key made here, or leaves
                // a vault's stick at no place, is not made into a
                // checklist (`docs/NEW-WALLET.md` §14.2, §14.4).
                if self.backup_kept_nowhere().is_some() || self.backup_stickless().is_some() {
                    return;
                }
                self.backup_extras();
                self.plan_save();
                self.backup_files_make();
                let items = self.backup_items();
                let first = items
                    .iter()
                    .copied()
                    .find(|&n| bstep::item(n).is_some_and(|it| !self.backup_item_done(it)));
                if let Some(b) = self.backup.as_mut() {
                    b.stage = BStage::Checklist;
                    b.naming = None;
                    b.scroll = flow::Scroll::default();
                }
                self.backup_item_open(first);
                self.backups_sync();
            }
            Action::BPlan => {
                // A map edited on the chart is the plan: changing it from
                // the questions asks first (`docs/NEW-WALLET.md` §9.7).
                if let Some(w) = self
                    .backup
                    .as_ref()
                    .filter(|b| b.answers.map.is_some())
                    .map(|b| b.wallet)
                {
                    self.chart_act(glance_sheet::ChartAction::Open(
                        glance::Press::Loaded(w),
                        glance_sheet::Target::Replan,
                    ));
                    return;
                }
                // What the checklist has put in vaults so far, should the
                // new plan drop it.
                self.backup_prior();
                if let Some(b) = self.backup.as_mut() {
                    b.stage = BStage::Plan;
                    b.q = None;
                    b.checking = false;
                    b.scroll = flow::Scroll::default();
                }
            }
            Action::BWallets => {
                if let Some(b) = self.backup.as_mut() {
                    b.pick = !b.pick;
                }
            }
            Action::BStep(n) => {
                let open = self.backup.as_ref().and_then(|b| b.open);
                // An item opens once every item before it is done
                // (`docs/NEW-WALLET.md` §14.1).
                if open == Some(n) {
                    self.backup_item_open(None);
                } else if self.backup_reachable(n) {
                    self.backup_item_open(Some(n));
                }
            }
            Action::BNext(n) => {
                // No way past an item but doing it; the envelopes are done
                // by this press.
                if n != bstep::ENVELOPE
                    && !bstep::item(n).is_some_and(|it| self.backup_item_done(it))
                {
                    return;
                }
                let items = self.backup_items();
                let next = if n == bstep::ENVELOPE {
                    if let Some(b) = self.backup.as_mut() {
                        b.envelopes = true;
                    }
                    items
                        .iter()
                        .copied()
                        .find(|&i| bstep::item(i).is_some_and(|it| !self.backup_item_done(it)))
                } else {
                    items.iter().skip_while(|&&i| i != n).nth(1).copied()
                };
                self.backup_item_open(next);
            }
            Action::BKey(k) => {
                if let Some(b) = self.backup.as_mut() {
                    b.key = k;
                    b.reveal = false;
                    b.typed.clear();
                    b.counted = false;
                    b.scanned = None;
                    b.paper = None;
                }
            }
            Action::BReveal => {
                if let Some(b) = self.backup.as_mut() {
                    b.reveal = !b.reveal;
                }
            }
            Action::BXor(n) => self.paper_xor(n),
            Action::BCodex32(k, n) => self.paper_codex32(k, n),
            Action::BPaperHide => {
                if let Some(b) = self.backup.as_mut() {
                    b.paper = None;
                }
            }
            Action::BCompact(c) => {
                if let Some(b) = self.backup.as_mut() {
                    b.compact = c;
                    b.pin = 0;
                }
            }
            Action::BPin(r) => {
                if let Some(b) = self.backup.as_mut() {
                    b.pin = r;
                }
            }
            Action::BCheck => {
                if let Some(b) = self.backup.as_mut() {
                    b.checking = !b.checking;
                }
            }
            Action::BScan => {
                let fp = self
                    .backup
                    .as_ref()
                    .and_then(|b| self.session.keys.get(b.key))
                    .map(|k| k.master.fingerprint());
                if let Some(fp) = fp {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::CheckCopy(fp);
                    self.scan = Some(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.commands.push_back(Command::CameraOn);
                }
            }
            Action::BCheckClear => {
                if let Some(b) = self.backup.as_mut() {
                    b.typed.clear();
                    b.counted = false;
                }
            }
            Action::BVaultSave(v) => self.backup_vault_save(usize::from(v)),
            Action::BFilesMake => {
                self.backup_files_make();
                self.backups_sync();
            }
            Action::BackupFirst(w) => self.backup_first(w),
            Action::BNewVault => {
                self.vault_lock_one(self.vaults.current);
                self.vault_act(vaults::VaultAction::CreateFrom(Screen::Backup));
            }
            Action::BFile => {
                // The seeds' files item makes the first one not yet made.
                let next = self.backup.as_ref().and_then(|b| {
                    let stem = file_stem(&self.session.wallets.get(b.wallet)?.name);
                    self.backup_keys(b.wallet)
                        .into_iter()
                        .find(|&k| !self.seed_file_made(&stem, &self.session.keys[k]))
                });
                if let (Some(k), Some(b)) = (next, self.backup.as_mut())
                    && b.open == Some(bstep::FILES)
                {
                    b.key = k;
                }
                self.offer_seed();
            }
            Action::BWords(n) => {
                // On the checklist the template For the stick is made
                // again for the new length.
                let remake = self.backup_remake(0);
                if let Some(b) = self.backup.as_mut() {
                    b.words = n;
                }
                if let Some(w) = remake {
                    self.public_out(w, 0);
                }
            }
            Action::BOmit(n) => {
                let remake = self.backup_remake(4);
                if let Some(b) = self.backup.as_mut() {
                    b.answers.omit = n;
                }
                if let Some(w) = remake {
                    self.public_out(w, 4);
                }
            }
            Action::BOut(what) => self.backup_out(what),
            Action::Catalog(i) => self.catalog_go(usize::from(i)),
            Action::InboxLoad => {
                let (w, k) = self.inbox_load();
                self.toast(&format!(
                    "{w} {} and {k} {} loaded",
                    if w == 1 { "wallet" } else { "wallets" },
                    if k == 1 { "seed" } else { "seeds" }
                ));
            }
            Action::InboxChoose(i) => {
                if let Some(w) = self.inbox_found().to_load().get(i) {
                    let d = w.descriptor.clone();
                    if !self.inbox_skip.remove(&d) {
                        self.inbox_skip.insert(d);
                    }
                }
            }
            Action::PotentialOpen(fp) => {
                if self.may_load_keys() {
                    self.potential_open(osk_bip::keys::Fingerprint(fp));
                } else {
                    self.toast(&format!("Remove the {} first", self.medium.noun()));
                }
            }
            Action::XpubOpen(i) => self.xpub_open(i),
            Action::BackupOpen(k) => {
                if self.may_load_keys() {
                    self.backup_open(k);
                } else {
                    self.toast(&format!("Remove the {} first", self.medium.noun()));
                }
            }
            Action::PotentialKind(k) => {
                if let (Some(p), Some(&kind)) =
                    (self.potential.as_mut(), inbox::KINDS.get(usize::from(k)))
                {
                    p.kind = kind;
                }
            }
            Action::PotentialShow => {
                if let Some(p) = self.potential.as_mut() {
                    p.shown = !p.shown;
                }
            }
            Action::PotentialType => {
                if let Some(p) = self.potential.as_mut() {
                    p.typing = self.sticks.is_empty();
                }
            }
            Action::PotentialMake => self.potential_make(),
            Action::DecodeFinished => {
                if let Some(s) = self.spend.as_ref()
                    && let Some(tx) = s.spend.finished.as_ref()
                {
                    let prevouts = decode::prevouts_of(&s.spend.psbt);
                    let source = format!("Finished from {}", s.spend.source);
                    let decoded = decode::decode(tx, &prevouts, &self.session, &source);
                    self.decode = Some(DecodeState {
                        decoded,
                        back: self.screen,
                        show_hex: false,
                    });
                    self.screen = Screen::Decode;
                }
            }
            Action::DecodeInbox(k) => {
                if let Some(item) = self.inbox.get(k)
                    && let Some(tx) = osk_psbt::transaction::read_raw(&item.bytes)
                {
                    // The amounts going in, from a PSBT of the same
                    // transaction when one is here.
                    let txid = tx.compute_txid();
                    let psbt = self
                        .spend
                        .as_ref()
                        .map(|s| s.spend.psbt.clone())
                        .filter(|p| p.unsigned_tx().compute_txid() == txid)
                        .or_else(|| {
                            self.inbox
                                .iter()
                                .filter(|i| i.kind == FileKind::Psbt)
                                .filter_map(|i| wallet::read_psbt(&i.bytes))
                                .find(|p| p.unsigned_tx().compute_txid() == txid)
                        });
                    let prevouts = psbt.map(|p| decode::prevouts_of(&p)).unwrap_or_default();
                    let decoded = decode::decode(&tx, &prevouts, &self.session, &item.name);
                    self.decode = Some(DecodeState {
                        decoded,
                        back: self.screen,
                        show_hex: false,
                    });
                    self.screen = Screen::Decode;
                    self.list_offset = 0.0;
                }
            }
            Action::QrDecoded => {
                if let Some(d) = self.decode.as_ref() {
                    let hex = d.decoded.hex.clone();
                    self.open_qr(QrView::text("Transaction", &hex));
                }
            }
            Action::DecodeHex => {
                if let Some(d) = self.decode.as_mut() {
                    d.show_hex = !d.show_hex;
                }
            }
            Action::SpendHex => {
                if let Some(s) = self.spend.as_mut() {
                    s.show_hex = !s.show_hex;
                }
            }
            Action::WriteAsk => {
                if !self.clean() {
                    self.sheet = Some(Sheet::WriteOut);
                } else if self.sticks.is_empty() {
                    self.toast(&format!(
                        "Plug in {}: the visit writes what waits for it",
                        self.medium.a()
                    ));
                } else {
                    self.act(Action::Nav(Screen::Visit));
                }
            }
            Action::SignMessage => {
                let first = self.session.message_wallets().first().copied();
                self.message = Some(MessageState {
                    open: Some(mstep::ADDRESS),
                    wallet: first,
                    bip322: true,
                    ..MessageState::default()
                });
                self.screen = Screen::Message;
            }
            Action::MStep(n) => {
                if let Some(m) = self.message.as_mut() {
                    m.open = if m.open == Some(n) { None } else { Some(n) };
                    m.scroll.follow = true;
                    m.typing = m.open == Some(mstep::TEXT);
                }
            }
            Action::MNext(n) => {
                if let Some(m) = self.message.as_mut() {
                    m.done[n as usize] = true;
                    m.open = (0..4u8).find(|&i| !m.done[i as usize]);
                    m.scroll.follow = true;
                    m.typing = m.open == Some(mstep::TEXT);
                }
            }
            Action::MWallet(i) => {
                if let Some(m) = self.message.as_mut() {
                    m.wallet = Some(i);
                    m.index = 0;
                    m.signed = None;
                }
            }
            Action::MIndex(d) => {
                if let Some(m) = self.message.as_mut() {
                    m.index = (i64::from(m.index) + i64::from(d)).clamp(0, 9999) as u32;
                    m.signed = None;
                }
            }
            Action::MType => {
                if let Some(m) = self.message.as_mut() {
                    m.typing = !m.typing;
                }
            }
            Action::MFormat(b) => {
                if let Some(m) = self.message.as_mut() {
                    m.bip322 = b;
                    m.signed = None;
                }
            }
            Action::MSign => {
                if let Some(m) = self.message.as_mut()
                    && let Some(w) = m.wallet
                {
                    let format = if m.bip322 {
                        osk_psbt::message::Format::Bip322
                    } else {
                        osk_psbt::message::Format::Bip137
                    };
                    match self.session.sign_message(w, m.index, &m.text, format) {
                        Ok(signed) => {
                            m.signed = Some(signed);
                            m.error = None;
                            m.done[mstep::SIGN as usize] = true;
                        }
                        Err(e) => m.error = Some(e),
                    }
                }
                // The signed message goes For the stick as it is made.
                if self.message.as_ref().is_some_and(|m| m.signed.is_some()) {
                    self.act(Action::MOut);
                }
            }
            Action::MOut | Action::MQr => {
                if let Some(m) = self.message.as_ref()
                    && let Some(sig) = m.signed.as_ref()
                {
                    let text =
                        osk_psbt::message::signed_text(&sig.address, &sig.signature, &m.text);
                    if action == Action::MOut {
                        let name = format!(
                            "message-{}.txt",
                            &sig.address[sig.address.len().saturating_sub(6)..]
                        );
                        self.put_outbox(&name, text.into_bytes());
                        self.toast_out(&name);
                    } else {
                        let view = self.code_view(Code::Message);
                        self.open_qr(view);
                    }
                }
            }
            Action::CreateWallet if self.create_unfinished() => {
                self.screen = Screen::Create;
            }
            Action::CreateWallet | Action::CreateOver => {
                // Create opens with Kind open, whatever opened it
                // (`docs/NEW-WALLET.md` §2.1).
                self.create = Some(CreateState {
                    open: Some(cstep::KIND),
                    m: 2,
                    n: 3,
                    slots: vec![create::Source::Empty; 1],
                    ..CreateState::default()
                });
                self.screen = Screen::Create;
            }
            Action::CStep(k) => {
                if let Some(c) = self.create.as_mut() {
                    c.open = if c.open == Some(k) { None } else { Some(k) };
                    c.scroll.follow = true;
                }
            }
            // Keys' Continue makes the wallet, which opens Check; until
            // it is made, a refusal keeps Keys open with the reason.
            Action::CNext(k)
                if k == cstep::KEYS && self.create.as_ref().is_some_and(|c| c.built.is_none()) =>
            {
                self.create_make();
            }
            Action::CNext(k) => {
                if let Some(c) = self.create.as_mut() {
                    c.done[k as usize] = true;
                    let steps = create_steps(c.kind);
                    c.open = steps.iter().copied().find(|&i| !c.done[i as usize]);
                    c.scroll.follow = true;
                }
                // Check's own Continue: this wallet is checked this
                // power-on (`docs/WALLETS.md` §4).
                if k == cstep::CHECK
                    && let Some(w) = self.create.as_ref().and_then(|c| c.built)
                    && let Some(wallet) = self.session.wallets.get(w)
                {
                    let checksum = wallet.policy.checksum();
                    self.mark_checked(&checksum);
                }
            }
            Action::CKind(i) => {
                if let Some(c) = self.create.as_mut()
                    && c.built.is_none()
                {
                    // Ticks the kind, whether chosen by hand on an open
                    // Kind card or by a Tools tile opening Create with
                    // its kind already ticked (`docs/NEW-WALLET.md`
                    // §2.1): either way Kind stays open until its own
                    // Continue closes it.
                    c.kind = create::NewKind::ALL[i as usize % create::NewKind::ALL.len()];
                    if c.kind.multi() {
                        c.n = c.n.clamp(2, c.kind.max_keys());
                        c.m = if c.kind.all_sign() { c.n } else { c.m.min(c.n) };
                    }
                    // A threshold wallet's slots are the shares chosen;
                    // the deal computes the rest.
                    let n = match c.kind {
                        k if k.threshold() => c.m,
                        k if k.multi() => c.n,
                        _ => 1,
                    };
                    if c.kind.threshold() {
                        c.slots.retain(|s| matches!(s, create::Source::Here(_)));
                    }
                    c.slots.resize(n, create::Source::Empty);
                    c.slots.truncate(n);
                }
            }
            Action::CBackup(k) => {
                if let Some(i) = self.create.as_ref().and_then(|c| c.built) {
                    self.act(Action::Backup(i));
                    if let Some(b) = self.backup.as_mut() {
                        b.from_create = true;
                    }
                    self.act(Action::BPreset(k));
                }
            }
            Action::CMoreKinds => {
                if let Some(c) = self.create.as_mut() {
                    c.more_kinds = true;
                }
            }
            Action::CM(d) => {
                if let Some(c) = self.create.as_mut() {
                    c.m = if c.kind.all_sign() {
                        c.n
                    } else {
                        (c.m as i64 + i64::from(d)).clamp(1, c.n as i64) as usize
                    };
                    if c.kind.threshold() {
                        c.slots.resize(c.m, create::Source::Empty);
                    }
                }
            }
            Action::CN(d) => {
                if let Some(c) = self.create.as_mut()
                    && c.kind.multi()
                {
                    c.n = (c.n as i64 + i64::from(d)).clamp(2, c.kind.max_keys() as i64) as usize;
                    c.m = if c.kind.all_sign() { c.n } else { c.m.min(c.n) };
                    let slots = if c.kind.threshold() { c.m } else { c.n };
                    c.slots.resize(slots, create::Source::Empty);
                }
            }
            Action::CSlotHere(k, fp) => {
                if let Some(c) = self.create.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Here(fp);
                    if c.pass_slot == Some(k) {
                        c.close_pass();
                    }
                }
            }
            Action::CSlotFile(k, i) => {
                let kind = self.create.as_ref().map(|c| c.kind).unwrap_or_default();
                let key = self
                    .inbox
                    .get(i)
                    .and_then(|it| create::key_for(kind, &String::from_utf8_lossy(&it.bytes)));
                // A cosigner's key sets the network the wallet is made on.
                let net = key
                    .as_deref()
                    .and_then(|k| osk_bip::policy::PolicyKey::parse(k).ok())
                    .map(|k| wallet::network_of_key(&k, self.session.network()));
                if let Err(e) = self.session.follow(net) {
                    if let Some(c) = self.create.as_mut() {
                        c.error = Some(e.text());
                    }
                    return;
                }
                if let Some(c) = self.create.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                    && let Some(key) = key
                {
                    *s = create::Source::Cosigner(key);
                }
            }
            Action::CSlotLater(k) => {
                if let Some(c) = self.create.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Later;
                }
            }
            Action::CKeyOut(k) | Action::CKeyQr(k) => {
                let qr = matches!(action, Action::CKeyQr(_));
                match self.create_key_text(k) {
                    Ok((fp, text)) => {
                        let kind = self.create.as_ref().map_or("", |c| c.kind.name());
                        if qr {
                            let view = self.code_view(Code::Key(k));
                            self.open_qr(view);
                        } else {
                            let name = format!("xpub-{fp}.txt");
                            self.put_outbox(
                                &name,
                                format!("# Account xpub {fp}, {kind}\n{text}\n").into_bytes(),
                            );
                            self.toast_out(&name);
                        }
                    }
                    Err(e) => self.toast(&e),
                }
            }
            Action::CKeyBsms(k) => match self.create_key_bsms(k) {
                Some((fpt, text)) => {
                    let name = format!("xpub-{fpt}-bsms.txt");
                    self.put_outbox(&name, text.into_bytes());
                    self.toast_out(&name);
                }
                None => self.toast("BIP 129 covers wsh and sh(wsh) multisig keys"),
            },
            Action::CSlotClear(k) => {
                if let Some(c) = self.create.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Empty;
                    if c.pass_slot == Some(k) {
                        c.close_pass();
                    }
                }
            }
            Action::CPassOpen(k) => {
                if let Some(c) = self.create.as_mut() {
                    if c.pass_slot == Some(k) {
                        c.close_pass();
                    } else if c.built.is_none() {
                        c.close_pass();
                        c.pass_slot = Some(k);
                        c.pass_focus = Some(0);
                    }
                }
            }
            Action::CPassField(f) => {
                if let Some(c) = self.create.as_mut()
                    && c.pass_slot.is_some()
                    && f < 2
                {
                    c.pass_focus = Some(f);
                    c.pass_note = None;
                }
            }
            Action::CPassLock => self.create_pass_lock(),
            Action::Rename => {
                // Pressing the name field again keeps what is typed.
                if self.renaming.is_none() {
                    self.renaming = self
                        .session
                        .wallets
                        .get(self.wallet)
                        .map(|w| w.name.clone());
                }
            }
            Action::ScanPart => {
                if self.may_load_keys() {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::KeyPart;
                    self.scan = Some(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.commands.push_back(Command::CameraOn);
                }
            }
            Action::ScanSeed => {
                if self.may_load_keys() {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::Seed;
                    self.scan = Some(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.commands.push_back(Command::CameraOn);
                }
            }
            Action::Scan => {
                let mut scan = ScanState::default();
                if let Some((decoder, assembler)) = self.scan_parts.take() {
                    scan.decoder = decoder;
                    scan.assembler = assembler;
                    scan.note = Some("Carrying on with the parts read before".to_string());
                }
                self.scan = Some(scan);
                self.sheet = Some(Sheet::Scan);
                self.commands.push_back(Command::CameraOn);
            }
            Action::RestoreWallet => {
                self.restore = Some(RestoreState::new());
                self.screen = Screen::Restore;
            }
            Action::RStep(k) => {
                if let Some(r) = self.restore.as_mut() {
                    r.open = if r.open == Some(k) { None } else { Some(k) };
                    r.pass_focus = None;
                    r.scroll.follow = true;
                }
            }
            Action::RNext(k) => {
                if let Some(r) = self.restore.as_mut() {
                    r.pass_focus = None;
                    r.next(k);
                }
            }
            Action::RKind(i) => self.restore_kind(i),
            Action::RMoreKinds => {
                if let Some(r) = self.restore.as_mut() {
                    r.more_kinds = true;
                }
            }
            Action::RFromFiles => {
                let files: Vec<usize> = self
                    .inbox
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.kind == FileKind::Wallet)
                    .map(|(k, _)| k)
                    .collect();
                match files.as_slice() {
                    [one] => self.act(Action::RUse(*one)),
                    _ => {
                        if let Some(r) = self.restore.as_mut() {
                            r.open = Some(rstep::DESCRIPTION);
                            r.scroll.follow = true;
                        }
                    }
                }
            }
            Action::RAbsent(k) => {
                if let Some(r) = self.restore.as_mut()
                    && r.described
                {
                    match r.absent.iter().position(|&a| a == k) {
                        Some(p) => {
                            r.absent.remove(p);
                        }
                        None => r.absent.push(k),
                    }
                    r.error = None;
                }
            }
            Action::RPassField(f) => {
                if let Some(r) = self.restore.as_mut()
                    && f < 2
                {
                    r.pass_focus = Some(f);
                    r.error = None;
                    if let Some(s) = r.seeds.as_mut() {
                        s.focus = None;
                    }
                }
            }
            Action::RSlotWords => {
                if self.restore_pass_ok() {
                    self.act(Action::Entry(None));
                    if self.screen == Screen::Entry
                        && let Some(r) = self.restore.as_ref()
                    {
                        self.entry.passphrase.set(r.pass.as_str());
                    }
                }
            }
            Action::RSlotScan => {
                if self.restore_pass_ok() && self.may_load_keys() {
                    self.entry = EntryState::default();
                    self.entry.back = Some(Screen::Restore);
                    if let Some(r) = self.restore.as_ref() {
                        self.entry.passphrase.set(r.pass.as_str());
                    }
                    self.act(Action::ScanSeed);
                }
            }
            Action::RUse(i) => {
                if let Some(item) = self.inbox.get(i) {
                    let text = String::from_utf8_lossy(&item.bytes).into_owned();
                    let name = osk_bip::multisig_config::parse_named(&text)
                        .ok()
                        .and_then(|(_, n)| n)
                        .unwrap_or_else(|| stem(&item.name));
                    let source = item.name.clone();
                    self.restore_wallet(&name, &text, &source);
                }
            }
            Action::RShare(i) => {
                if let Some(item) = self.inbox.get(i) {
                    let n = item.name.clone();
                    if let Some(r) = self.restore.as_mut() {
                        if let Some(p) = r.shares.iter().position(|x| *x == n) {
                            r.shares.remove(p);
                        } else {
                            r.shares.push(n);
                        }
                    }
                }
            }
            Action::RRebuild => {
                let texts = self.restore_share_texts();
                match restore::merge(&texts) {
                    Ok(m) => match m.whole {
                        Some(whole) => {
                            let name = osk_bip::multisig_config::parse_named(&whole)
                                .ok()
                                .and_then(|(_, n)| n)
                                .unwrap_or_else(|| "Restored wallet".to_string());
                            self.restore_wallet(&name, &whole, "its shares");
                        }
                        None => {
                            if let Some(r) = self.restore.as_mut() {
                                r.error = Some(format!("{} of {} keys in hand", m.have.len(), m.n));
                            }
                        }
                    },
                    Err(e) => {
                        if let Some(r) = self.restore.as_mut() {
                            r.error = Some(e);
                        }
                    }
                }
            }
            Action::RSeeds => {
                if let Some(r) = self.restore.as_mut()
                    && !r.kind().threshold()
                {
                    // The wallet comes from the seeds now, not a file.
                    r.wallet = None;
                    r.described = false;
                    r.absent.clear();
                    r.error = None;
                    r.done[rstep::SEEDS as usize] = false;
                    r.done[rstep::CHECK as usize] = false;
                    r.next(rstep::DESCRIPTION);
                }
            }
            Action::KeyWallet(fp, n) => {
                use create::NewKind;
                let mut s = seeds::SeedsState::default();
                s.fix(if n > 1 {
                    NewKind::Multi
                } else {
                    NewKind::NativeSegwit
                });
                s.set_n(usize::from(n));
                s.take(fp);
                let mut r = RestoreState {
                    seeds: Some(s),
                    from_key: true,
                    ..RestoreState::default()
                };
                for k in [rstep::KIND, rstep::QUORUM, rstep::DESCRIPTION] {
                    r.done[k as usize] = true;
                }
                r.open = Some(rstep::SEEDS);
                self.restore = Some(r);
                self.screen = Screen::Restore;
            }
            Action::Seeds(a) => {
                if self.screen == Screen::Restore
                    && let Some(r) = self.restore.as_mut()
                {
                    r.pass_focus = None;
                }
                self.seeds_act(a)
            }
            Action::Slide(id, v) => self.slide(id, v),
            Action::RestoreShares => {
                let texts: Vec<String> = self
                    .inbox
                    .iter()
                    .filter(|i| i.kind == FileKind::Share)
                    .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
                    .collect();
                match restore::merge(&texts).map(|m| m.whole) {
                    Ok(Some(whole)) => {
                        let name = osk_bip::multisig_config::parse_named(&whole)
                            .ok()
                            .and_then(|(_, n)| n)
                            .unwrap_or_else(|| "Restored wallet".to_string());
                        match self.session.add_wallet(&name, &whole, "its shares") {
                            Ok(i) => {
                                self.wallet = i;
                                self.screen = Screen::Wallets;
                                self.refresh_spend();
                                self.toast(&format!("{name} restored from its shares"));
                            }
                            Err(e) => self.toast(&e.text()),
                        }
                    }
                    Ok(None) => self.toast("Not every key is in hand yet"),
                    Err(e) => self.toast(&e),
                }
            }
            Action::CheckMessage(i) => {
                if i < self.inbox.len() {
                    self.checking = Some(i);
                    self.screen = Screen::CheckMessage;
                }
            }
            Action::PdfInbox(i) => {
                if let Some(item) = self.inbox.get(i) {
                    let (name, bytes) = (item.name.clone(), item.bytes.clone());
                    self.make_pdf(&name, &bytes);
                }
            }
            Action::PdfOutbox(i) => {
                if let Some(item) = self.outbox.get(i) {
                    let (name, bytes) = (item.name.clone(), item.bytes.clone());
                    self.make_pdf(&name, &bytes);
                }
            }
            Action::QrSigned | Action::QrPart => {
                if let Some(s) = self.spend.as_ref() {
                    let title = if action == Action::QrSigned {
                        "Signed PSBT"
                    } else {
                        "PSBT for the next signer"
                    };
                    let bytes = s.spend.psbt.to_bytes();
                    self.open_qr(QrView::psbt(title, &bytes));
                }
            }
            Action::QrWallet(i) => {
                if let Some(b) = self.backup.as_mut()
                    && b.wallet == i
                    && self.screen == Screen::Backup
                {
                    b.shown = true;
                    self.backups_shown();
                }
                if i < self.session.wallets.len() {
                    let view = self.code_view(Code::Descriptor(i));
                    self.open_qr(view);
                }
            }
            Action::ShowCode(c) => {
                let view = self.code_view(c);
                self.open_qr(view);
            }
            Action::CodePng(c) => {
                let view = self.code_view(c);
                self.code_png(view.as_ref());
            }
            Action::QrPng => {
                if let Some(q) = self.qr.take() {
                    self.code_png(Ok(&q));
                    self.qr = Some(q);
                }
            }
            Action::QrOutbox(i) => {
                if let Some(item) = self.outbox.get(i).filter(|it| qr_fits(it)) {
                    // What a wallet reads goes as itself; any other file in
                    // the Faraday file envelope.
                    let source = transfer::qr_source(&item.name, &item.bytes, item.kind);
                    let secret = item.secret;
                    let view =
                        QrView::of(&item.name, source, QrFormat::Ur, QR_PARTS[1]).map(|mut v| {
                            v.secret |= secret;
                            v
                        });
                    self.open_qr(view);
                }
            }
            Action::QrFormat(i) => {
                if let Some(q) = self.qr.as_ref() {
                    let f = QrFormat::ALL[usize::from(i) % QrFormat::ALL.len()];
                    let v = q.redo(f, q.part);
                    self.open_qr(v);
                }
            }
            Action::QrPartSize(n) => {
                if let Some(q) = self.qr.as_ref() {
                    let v = q.redo(q.format, n);
                    self.open_qr(v);
                }
            }
            Action::QrSpeed(ms) => {
                self.qr_frame_ms = ms;
                self.save_boxes();
            }
            Action::TOther(id) => {
                if let Some(s) = self.spend.as_mut() {
                    if let Some(k) = s.others.iter().position(|o| *o == id) {
                        s.others.remove(k);
                    } else {
                        s.others.push(id);
                        s.others.sort_unstable();
                    }
                    s.error = None;
                }
            }
            Action::CarryToOutbox => {
                // The carry holds the secret nonce the next share signs
                // with: a vault by default, the Outbox only after a warning.
                let out = self.spend.as_mut().and_then(|s| {
                    let stem = result_stem(&s.spend.source);
                    let bytes = s.carry_out.take()?;
                    let psbt = s.spend.psbt.to_bytes();
                    let round = secrets::Round {
                        tx: secrets::tx_hash(&s.spend.psbt),
                        key: s.signers.first().copied().unwrap_or_default(),
                        psbt,
                        psbt_name: format!("{stem}-part.psbt"),
                    };
                    let mut out = secrets::SecretOut::new(
                        format!("{stem}-partly-signed.osk"),
                        bytes,
                        "The secret nonce the next share signs this transaction with",
                        "Whoever has it and a share's signature can work out that share",
                    );
                    out.keep = secrets::Keep::Round(round);
                    Some(out)
                });
                if let Some(out) = out {
                    self.offer_secret(out);
                }
            }
            Action::Scale(pct) => {
                self.scale_pct = pct;
                if let Some(d) = self.last_display {
                    self.display(d);
                }
                self.save_boxes();
            }
            Action::ReduceMotion(on) => {
                self.reduce_motion = on;
                self.motion.stop();
                self.save_boxes();
            }
            Action::Theme(theme) => {
                self.theme = theme;
                self.save_boxes();
            }
        }
    }

    /// The session keys that sign for wallet `i`, by index into the
    /// session, that came from words.
    pub fn backup_keys(&self, i: usize) -> Vec<usize> {
        let Some(w) = self.session.wallets.get(i) else {
            return Vec::new();
        };
        self.backup_keys_of(w)
    }

    /// [`Faraday::backup_keys`] for a wallet that need not be loaded.
    pub(crate) fn backup_keys_of(&self, w: &wallet::Wallet) -> Vec<usize> {
        // A threshold wallet's keys here are the shares it lists.
        if let Some(record) = w.policy.record() {
            return self
                .session
                .keys
                .iter()
                .enumerate()
                .filter(|(_, k)| {
                    k.words.is_some()
                        && k.share
                            .is_some_and(|p| record.info.pubshares.contains(&Some(p)))
                })
                .map(|(i, _)| i)
                .collect();
        }
        let fps: Vec<_> = w
            .policy
            .keys()
            .iter()
            .filter_map(|k| k.fingerprint())
            .collect();
        self.session
            .keys
            .iter()
            .enumerate()
            .filter(|(_, k)| k.words.is_some() && fps.contains(&k.master.fingerprint()))
            .map(|(i, _)| i)
            .collect()
    }

    /// Where the wallet being backed up and each of its keys are kept, as
    /// the small panel's copy page lists them: in which open vault, whether the copy by hand was
    /// checked, whether a file of it is in the Outbox unprotected, or not
    /// here at all.
    pub fn backup_kept(&self) -> Option<backup::Kept> {
        use crate::vault_screens::{vault_has_wallet, vault_key};
        use backup::{KeptSeed, Tone};
        let b = self.backup.as_ref()?;
        let wallet = self.session.wallets.get(b.wallet)?;
        let open = 0..self.vaults.open.len();
        // A locked vault cannot be looked into.
        let unknown = self.vaults.open.is_empty() && !self.vault_files().is_empty();
        let not_known = || ("Not checked: no vault unlocked".to_string(), Tone::Dim);
        let wallet_line = match open.clone().find(|&v| vault_has_wallet(self, v, wallet)) {
            Some(v) => (format!("Wallet in {}", self.vaults.open[v].name), Tone::Ok),
            None if unknown => not_known(),
            None => ("Wallet not in a vault".to_string(), Tone::Warn),
        };
        let stem = file_stem(&wallet.name);
        let here = self.backup_keys(b.wallet);
        let seed_lines = |key: &wallet::Key| -> Vec<(String, Tone)> {
            let fp = key.master.fingerprint();
            let mut lines = Vec::new();
            lines.push(
                match open
                    .clone()
                    .find_map(|v| vault_key(self, v, fp).map(|p| (v, p)))
                {
                    Some((v, with)) => (
                        format!(
                            "In {}{}",
                            self.vaults.open[v].name,
                            if with { " with its passphrase" } else { "" }
                        ),
                        Tone::Ok,
                    ),
                    None if unknown => not_known(),
                    None => ("Not in a vault".to_string(), Tone::Warn),
                },
            );
            lines.push(if b.checked.contains(&fp.0) {
                ("Paper copy checked".to_string(), Tone::Ok)
            } else {
                ("Paper: not checked".to_string(), Tone::Warn)
            });
            let fps = fp_text(fp).to_lowercase();
            let files = ["words.txt", "seedqr.png", "compactseedqr.png"]
                .map(|end| format!("{stem}-{fps}-{end}"));
            if self
                .outbox
                .iter()
                .any(|i| i.secret && files.contains(&i.name))
            {
                lines.push((
                    format!("File {}, unprotected", self.medium.for_the()),
                    Tone::Err,
                ));
            }
            lines
        };
        let away = || {
            vec![(
                "Not here: backed up on its own device".to_string(),
                Tone::Dim,
            )]
        };
        let mut seeds = Vec::new();
        if let Some(record) = wallet.policy.record() {
            // A threshold wallet's keys are its shares, by number.
            for (id, pubshare) in record.info.pubshares.iter().enumerate() {
                let key = here
                    .iter()
                    .map(|&k| &self.session.keys[k])
                    .find(|k| pubshare.is_some() && k.share.as_ref() == pubshare.as_ref());
                seeds.push(match key {
                    Some(key) => KeptSeed {
                        name: fp_text(key.master.fingerprint()),
                        lines: seed_lines(key),
                    },
                    None => KeptSeed {
                        name: format!("Share {}", id + 1),
                        lines: away(),
                    },
                });
            }
        } else {
            let mut fps: Vec<osk_bip::keys::Fingerprint> = Vec::new();
            for f in wallet.policy.keys().iter().filter_map(|k| k.fingerprint()) {
                if !fps.contains(&f) {
                    fps.push(f);
                }
            }
            for fp in fps {
                let key = here
                    .iter()
                    .map(|&k| &self.session.keys[k])
                    .find(|k| k.master.fingerprint() == fp);
                seeds.push(KeptSeed {
                    name: fp_text(fp),
                    lines: key.map_or_else(away, seed_lines),
                });
            }
        }
        Some(backup::Kept {
            wallet: wallet_line,
            seeds,
        })
    }

    /// Wallet `w`'s seeds, in its order, as the plan and the panel list
    /// them: a name each (its fingerprint, or a share not here by number)
    /// and the session key that holds it, when its words are here.
    pub fn backup_seed_list(&self, w: usize) -> Vec<(String, Option<usize>)> {
        let Some(wallet) = self.session.wallets.get(w) else {
            return Vec::new();
        };
        self.backup_seed_list_of(wallet)
    }

    /// [`Faraday::backup_seed_list`] for a wallet that need not be loaded.
    pub(crate) fn backup_seed_list_of(
        &self,
        wallet: &wallet::Wallet,
    ) -> Vec<(String, Option<usize>)> {
        let here = self.backup_keys_of(wallet);
        if let Some(record) = wallet.policy.record() {
            return record
                .info
                .pubshares
                .iter()
                .enumerate()
                .map(|(id, pubshare)| {
                    let key = here.iter().copied().find(|&k| {
                        pubshare.is_some()
                            && self.session.keys[k].share.as_ref() == pubshare.as_ref()
                    });
                    match key {
                        Some(k) => (fp_text(self.session.keys[k].master.fingerprint()), Some(k)),
                        None => (format!("Share {}", id + 1), None),
                    }
                })
                .collect();
        }
        let mut fps: Vec<osk_bip::keys::Fingerprint> = Vec::new();
        for f in wallet.policy.keys().iter().filter_map(|k| k.fingerprint()) {
            if !fps.contains(&f) {
                fps.push(f);
            }
        }
        fps.into_iter()
            .map(|fp| {
                let key = here
                    .iter()
                    .copied()
                    .find(|&k| self.session.keys[k].master.fingerprint() == fp);
                (fp_text(fp), key)
            })
            .collect()
    }

    /// What wallet `w`'s plan is for: its quorum, its seeds and whether
    /// its description splits.
    pub fn plan_shape(&self, w: usize) -> plan::Shape {
        let Some(wallet) = self.session.wallets.get(w) else {
            return plan::Shape {
                m: 1,
                keys: 1,
                seeds: Vec::new(),
                splits: false,
            };
        };
        self.plan_shape_of(wallet)
    }

    /// [`Faraday::plan_shape`] for a wallet that need not be loaded.
    pub(crate) fn plan_shape_of(&self, wallet: &wallet::Wallet) -> plan::Shape {
        let (m, n) = Session::quorum(wallet);
        let seeds: Vec<plan::Seed> = self
            .backup_seed_list_of(wallet)
            .into_iter()
            .map(|(name, key)| plan::Seed {
                name,
                here: key.is_some(),
                passphrase: key
                    .and_then(|k| self.session.keys[k].passphrase.as_ref())
                    .is_some_and(|p| !p.is_empty()),
            })
            .collect();
        plan::Shape {
            m: m.clamp(1, seeds.len().max(1)),
            keys: n.max(1),
            seeds,
            splits: backup::splits(wallet),
        }
    }

    /// The questions of the backup's plan this wallet asks, in order: no
    /// seed question for a wallet with no seed here, the software only
    /// where the description goes to software or files, a passphrase's
    /// places only where a seed has one, and on a small panel the map as
    /// the last page.
    pub fn backup_questions(&self) -> Vec<u8> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let shape = self.plan_shape(b.wallet);
        let a = &b.answers;
        let mut v = vec![qstep::PRESET];
        if !shape.watch_only() {
            v.push(qstep::SEEDS);
        }
        v.push(qstep::PLACES);
        v.push(qstep::WALLET);
        if a.wallet[plan::wallet::SOFTWARE] || a.wallet[plan::wallet::FILES] {
            v.push(qstep::SOFTWARE);
        }
        if shape.seeds.iter().any(|s| s.here && s.passphrase) {
            v.push(qstep::PASSPHRASE);
        }
        if self.is_compact() {
            v.push(qstep::MAP);
        }
        v
    }

    /// The backup's checklist, by [`bstep`]: only what the plan needs.
    pub fn backup_items(&self) -> Vec<u8> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let shape = self.plan_shape(b.wallet);
        plan::checklist(&shape, &b.answers)
            .into_iter()
            .map(bstep::of)
            .collect()
    }

    /// Opens checklist item `n`, or closes them all. A seed's copy shows
    /// that seed.
    fn backup_item_open(&mut self, n: Option<u8>) {
        let shape = self.backup.as_ref().map(|b| self.plan_shape(b.wallet));
        let key = n
            .and_then(bstep::item)
            .and_then(|it| match it {
                plan::Item::Copy(i) => Some(i),
                // A vault's item shows its first seed.
                plan::Item::Vault(v) => {
                    let b = self.backup.as_ref()?;
                    b.answers.vault_seeds(shape.as_ref()?, v).first().copied()
                }
                _ => None,
            })
            .and_then(|i| {
                let w = self.backup.as_ref()?.wallet;
                self.backup_seed_list(w).get(i)?.1
            });
        if let Some(b) = self.backup.as_mut() {
            b.open = n;
            b.scroll.follow = true;
            b.checking = false;
            if let Some(k) = key
                && b.key != k
            {
                b.key = k;
                b.reveal = false;
                b.typed.clear();
                b.counted = false;
                b.scanned = None;
                b.paper = None;
            }
        }
    }

    /// The public files the plan's software calls for, as
    /// [`Faraday::public_out`] numbers their text: Sparrow the wallet
    /// file, Coldcard, Keystone and Passport the multisig config, Nunchuk
    /// the BSMS record, Bitcoin Core its import, each the descriptor where
    /// the wallet has no such file; "Not sure" the descriptor.
    pub fn backup_public(&self) -> Vec<u8> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return Vec::new();
        };
        Self::backup_public_for(w, &b.answers)
    }

    /// [`Faraday::backup_public`] for wallet `w` under answers `a`.
    pub(crate) fn backup_public_for(w: &wallet::Wallet, a: &plan::Answers) -> Vec<u8> {
        use osk_bip::policy::{Template, Wrapper};
        use plan::software as sw;
        let config = backup::multisig_config(w, None).is_some();
        let bsms = matches!(
            w.policy.template(),
            Template::Multi {
                wrapper: Wrapper::Wsh | Wrapper::ShWsh,
                ..
            }
        );
        let core = w.policy.silent().is_none();
        let mut v: Vec<u8> = Vec::new();
        let add = |v: &mut Vec<u8>, x: u8| {
            if !v.contains(&x) {
                v.push(x);
            }
        };
        if a.software[sw::SPARROW] {
            add(&mut v, 5);
        }
        if a.software[sw::HARDWARE] {
            add(&mut v, if config { 2 } else { 1 });
        }
        if a.software[sw::NUNCHUK] {
            add(&mut v, if bsms { 6 } else { 1 });
        }
        if a.software[sw::CORE] {
            add(&mut v, if core { 7 } else { 1 });
        }
        if a.software[sw::NOT_SURE] || v.is_empty() {
            add(&mut v, 1);
        }
        v
    }

    /// The files [`Faraday::backup_public`] makes, in the form chosen: the
    /// text, the labelled picture, or both; a file with no picture as its
    /// text.
    fn backup_public_names(&self) -> Vec<String> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        self.backup_public_names_for(b.wallet, &b.answers)
    }

    /// [`Faraday::backup_public_names`] for loaded wallet `wi` under
    /// answers `a`.
    pub(crate) fn backup_public_names_for(&self, wi: usize, a: &plan::Answers) -> Vec<String> {
        let Some(w) = self.session.wallets.get(wi) else {
            return Vec::new();
        };
        let stem = file_stem(&w.name);
        let (qr, text) = (a.form[plan::form::QR], a.form[plan::form::TEXT]);
        let mut out = Vec::new();
        for what in Self::backup_public_for(w, a) {
            let file = match what {
                1 => format!("{stem}-descriptor.txt"),
                2 => format!("{stem}-multisig-config.txt"),
                5 => format!("{stem}-wallet.json"),
                6 => format!("{stem}-bsms.txt"),
                _ => format!("{stem}-bitcoin-core.json"),
            };
            let picture = match what {
                1 => self.public_pictures(wi, 8).first().map(|p| p.name.clone()),
                2 => Some(format!("{stem}-multisig-config.png")),
                6 => Some(format!("{stem}-bsms.png")),
                _ => None,
            };
            match picture {
                Some(p) if qr => {
                    out.push(p);
                    if text {
                        out.push(file);
                    }
                }
                _ => out.push(file),
            }
        }
        out
    }

    /// Whether a checklist item is done, by what it does: its file For
    /// the stick or written, its seeds or the wallet in the open vault, the copy
    /// matched, the descriptor shown; the envelopes alone by a press.
    pub fn backup_item_done(&self, item: plan::Item) -> bool {
        let Some(b) = self.backup.as_ref() else {
            return false;
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return false;
        };
        let stem = file_stem(&w.name);
        // For the stick, or written by the last visit.
        let out = |name: &str| {
            self.outbox.iter().any(|i| i.name == name)
                || self.receipt.as_ref().is_some_and(|r| r.wrote(name))
        };
        let seeds = self.backup_seed_list(b.wallet);
        let here: Vec<(usize, &wallet::Key)> = seeds
            .iter()
            .enumerate()
            .filter_map(|(i, (_, k))| Some((i, self.session.keys.get((*k)?)?)))
            .collect();
        match item {
            plan::Item::Templates => out(&format!("blank-template-{}-words.pdf", b.words)),
            plan::Item::Copy(i) => here
                .iter()
                .find(|(j, _)| *j == i)
                .is_some_and(|(_, k)| b.checked.contains(&k.master.fingerprint().0)),
            plan::Item::Vault(v) => self.backup_vault_fits(v).is_some(),
            plan::Item::SeedFiles => {
                !here.is_empty() && here.iter().all(|(_, k)| self.seed_file_made(&stem, k))
            }
            plan::Item::Sheets => {
                if b.answers.split && backup::splits(w) {
                    let (_, n) = Session::quorum(w);
                    out(&format!("{stem}-share-1-of-{n}.pdf"))
                } else {
                    out(&format!("{stem}-backup.pdf"))
                }
            }
            plan::Item::PublicFiles => self.backup_public_names().iter().all(|n| out(n)),
            plan::Item::ShowDescriptor => b.shown,
            plan::Item::Envelopes => b.envelopes,
        }
    }

    /// Whether a file of key `k`'s seed, for the wallet whose files are
    /// named `stem`, is For the stick or was written by the last visit.
    pub(crate) fn seed_file_made(&self, stem: &str, k: &wallet::Key) -> bool {
        let fps = fp_text(k.master.fingerprint()).to_lowercase();
        ["words.txt", "seedqr.png", "compactseedqr.png"]
            .iter()
            .any(|end| {
                let name = format!("{stem}-{fps}-{end}");
                self.outbox.iter().any(|i| i.secret && i.name == name)
                    || self.receipt.as_ref().is_some_and(|r| r.wrote(&name))
            })
    }

    /// The fingerprints of the seeds the plan puts into vault `v`, and of
    /// those it puts into another vault and not this one.
    pub(crate) fn backup_vault_keys(
        &self,
        v: usize,
    ) -> (
        Vec<osk_bip::keys::Fingerprint>,
        Vec<osk_bip::keys::Fingerprint>,
    ) {
        let Some(b) = self.backup.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let shape = self.plan_shape(b.wallet);
        let list = self.backup_seed_list(b.wallet);
        let fp = |i: usize| {
            let k = list.get(i)?.1?;
            Some(self.session.keys.get(k)?.master.fingerprint())
        };
        let own = b.answers.vault_seeds(&shape, v);
        let mut others: Vec<usize> = b
            .answers
            .vaults_made(&shape)
            .into_iter()
            .filter(|&u| u != v)
            .flat_map(|u| b.answers.vault_seeds(&shape, u))
            .filter(|i| !own.contains(i))
            .collect();
        others.sort_unstable();
        others.dedup();
        (
            own.into_iter().filter_map(fp).collect(),
            others.into_iter().filter_map(fp).collect(),
        )
    }

    /// The session keys of the seeds the plan puts into vault `v`, each
    /// with whether its passphrase goes in with it, and those of the seeds
    /// it puts into another vault and not this one.
    pub(crate) fn backup_vault_seeds(&self, v: usize) -> (Vec<(usize, bool)>, Vec<usize>) {
        let Some(b) = self.backup.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let shape = self.plan_shape(b.wallet);
        let list = self.backup_seed_list(b.wallet);
        let own = b.answers.vault_seeds(&shape, v);
        let mut others: Vec<usize> = b
            .answers
            .vaults_made(&shape)
            .into_iter()
            .filter(|&u| u != v)
            .flat_map(|u| b.answers.vault_seeds(&shape, u))
            .filter(|i| !own.contains(i))
            .filter_map(|i| list.get(i)?.1)
            .collect();
        others.sort_unstable();
        others.dedup();
        let own = own
            .into_iter()
            .filter_map(|i| {
                let k = list.get(i)?.1?;
                let with = shape.seeds[i].passphrase && b.answers.pass_in_vault(i);
                Some((k, with))
            })
            .collect();
        (own, others)
    }

    /// Whether open vault `o` holds a seed the plan puts into another
    /// vault and not vault `v`: it is not offered for vault `v`.
    pub fn backup_vault_taken(&self, v: usize, o: usize) -> bool {
        let (_, others) = self.backup_vault_seeds(v);
        others.iter().any(|&k| self.vault_holds_seed(o, k, false))
    }

    /// The vault that does the plan's vault `v`: an open vault, or one
    /// locked since this power-on by what it was seen to hold, that holds
    /// every seed the plan puts into `v` (with its passphrase where the
    /// plan puts it there), the wallet where the plan puts it there, and
    /// no seed only another vault is to hold. Its name.
    pub(crate) fn backup_vault_fits(&self, v: usize) -> Option<String> {
        use crate::vault_screens::vault_has_wallet;
        let b = self.backup.as_ref()?;
        let w = self.session.wallets.get(b.wallet)?;
        let (own_seeds, other_seeds) = self.backup_vault_seeds(v);
        let wallet_too = b.answers.wallet_in_vault(v);
        if own_seeds.is_empty() && !wallet_too {
            return None;
        }
        let open = (0..self.vaults.open.len()).find(|&o| {
            own_seeds
                .iter()
                .all(|&(k, with)| self.vault_holds_seed(o, k, with))
                && !other_seeds
                    .iter()
                    .any(|&k| self.vault_holds_seed(o, k, false))
                && (!wallet_too || vault_has_wallet(self, o, w))
        });
        if let Some(o) = open {
            return Some(self.vaults.open[o].name.clone());
        }
        let (own, others) = self.backup_vault_keys(v);
        let sum = w.policy.checksum();
        let text = |f: &osk_bip::keys::Fingerprint| fp_text(*f);
        let salts: Vec<[u8; 32]> = self.vaults.open.iter().map(|o| o.header().salt).collect();
        self.vaults
            .summaries
            .iter()
            .filter(|s| !salts.contains(&s.salt))
            .find(|s| {
                own.iter().all(|f| s.keys.contains(&text(f)))
                    && !others.iter().any(|f| s.keys.contains(&text(f)))
                    && (!wallet_too || s.sums.contains(&sum))
            })
            .map(|s| s.file.clone())
    }

    /// What place `p` is called on screen: its name, kept in the vault, or
    /// "Place 1", "Place 2".
    pub fn place_name(&self, p: usize) -> String {
        self.backup
            .as_ref()
            .and_then(|b| b.names.get(p))
            .map(|n| n.trim())
            .filter(|n| !n.is_empty())
            .map_or_else(|| format!("Place {}", p + 1), str::to_string)
    }

    /// The text a vault keeps a wallet under, as Vaults saves it.
    fn wallet_text(w: &wallet::Wallet) -> String {
        match (w.policy.record(), w.policy.silent()) {
            (Some(r), _) => r.to_text(),
            (None, Some(s)) => s.to_text(),
            (None, None) => w.policy.to_descriptor_checksummed(),
        }
    }

    /// The plan an open vault keeps for wallet `w` (record type 11): its
    /// answers and the places' names. None when no open vault has one,
    /// or it is not for a wallet of this shape.
    fn plan_load(&self, w: usize, shape: &plan::Shape) -> Option<(plan::Answers, Vec<String>)> {
        self.plan_load_of(self.session.wallets.get(w)?, shape)
    }

    /// [`Faraday::plan_load`] for a wallet that need not be loaded.
    pub(crate) fn plan_load_of(
        &self,
        wallet: &wallet::Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)> {
        (0..self.vaults.open.len()).find_map(|v| self.plan_in_vault(v, wallet, shape))
    }

    /// The plan open vault `v` keeps for `wallet` (its type 11 record),
    /// read against `shape`, with the places' names.
    pub(crate) fn plan_in_vault(
        &self,
        v: usize,
        wallet: &wallet::Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)> {
        use faraday_vault::records::{field, kind};
        let want = wallet::same_wallet(&wallet.policy);
        let o = self.vaults.open.get(v)?;
        o.contents.of(kind::PLAN).find_map(|(_, r)| {
            let same = r
                .text(field::PLAN_WALLET)
                .and_then(|t| wallet::read_wallet(t).ok())
                .is_some_and(|p| wallet::same_wallet(&p) == want);
            if !same {
                return None;
            }
            let a = plan::Answers::from_text(shape, r.text(field::PLAN_ANSWERS)?)?;
            let names = r
                .fields
                .iter()
                .filter(|f| f.number == field::PLAN_PLACE)
                .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
                .collect();
            Some((a, names))
        })
    }

    /// Saves the backup's plan into the open vault as record type 11,
    /// over the one it kept for this wallet: the wallet, the answers, the
    /// places' names and what each holds. The names are kept nowhere
    /// else. The vault that already keeps a plan for the wallet is the one
    /// written, else the current one.
    fn plan_save(&mut self) {
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return;
        };
        let v = self
            .plan_vault_of(w)
            .filter(|_| self.vaults.open.get(self.vaults.current).is_some())
            .unwrap_or(self.vaults.current);
        let (wi, answers, names, extras) = (
            b.wallet,
            b.answers.clone(),
            b.names.clone(),
            b.extras.clone(),
        );
        self.plan_store(v, wi, &answers, &names, &extras, None);
    }

    /// The open vault that keeps a plan (record type 11) for `wallet`.
    pub(crate) fn plan_vault_of(&self, wallet: &wallet::Wallet) -> Option<usize> {
        use faraday_vault::records::{field, kind};
        let want = wallet::same_wallet(&wallet.policy);
        (0..self.vaults.open.len()).find(|&v| {
            self.vaults.open[v].contents.of(kind::PLAN).any(|(_, r)| {
                r.text(field::PLAN_WALLET)
                    .and_then(|t| wallet::read_wallet(t).ok())
                    .is_some_and(|p| wallet::same_wallet(&p) == want)
            })
        })
    }

    /// Writes loaded wallet `wi`'s plan into open vault `v` as record type
    /// 11, over the one it kept for the wallet: `answers`, the places'
    /// `names`, what each spot holds, work a later plan dropped
    /// (`extras`), and the notes on its map (§9.5): `notes`, or those the
    /// record kept.
    pub(crate) fn plan_store(
        &mut self,
        v: usize,
        wi: usize,
        answers: &plan::Answers,
        names: &[String],
        extras: &[String],
        notes: Option<&[plan::Note]>,
    ) {
        use faraday_vault::records::{Record, field, kind};
        let Some(w) = self.session.wallets.get(wi) else {
            return;
        };
        let Some(open) = self.vaults.open.get(v) else {
            return;
        };
        let shape = self.plan_shape(wi);
        // The keys made here with their backup pending, by fingerprint
        // (`docs/NEW-WALLET.md` §14.3).
        let mut text = answers.to_text();
        for fp in self.wallet_held(wi) {
            text.push_str(&format!("held {}\n", fp_text(fp)));
        }
        let mut record = Record::new(kind::PLAN)
            .with(field::PLAN_WALLET, Self::wallet_text(w).as_bytes())
            .with(field::PLAN_ANSWERS, text.as_bytes());
        for p in 0..answers.places {
            let name = names.get(p).map_or("", |n| n.trim());
            record.push(field::PLAN_PLACE, name.as_bytes());
        }
        let place = |p: usize| {
            names
                .get(p)
                .map(|n| n.trim())
                .filter(|n| !n.is_empty())
                .map_or_else(|| format!("Place {}", p + 1), str::to_string)
        };
        for spot in plan::map(&shape, answers) {
            let at = match spot.at {
                plan::At::Place(p) => place(p),
                plan::At::Vault(v) => answers.vault_name(&shape, v),
                plan::At::Files => format!("{} of files", self.medium.cap()),
                plan::At::Software => "Watch-only software".to_string(),
                plan::At::Away => "On its own device".to_string(),
            };
            let holds: Vec<String> = spot
                .holds
                .iter()
                .map(|(h, _)| h.label(&shape, self.medium))
                .collect();
            record.push(
                field::PLAN_HOLDS,
                format!("{at}: {}", holds.join(", ")).as_bytes(),
            );
        }
        // Work a later plan dropped stays on the map as what it is.
        for line in extras {
            record.push(field::PLAN_HOLDS, line.as_bytes());
        }
        let want = wallet::same_wallet(&w.policy);
        let same = |r: &faraday_vault::records::Record| {
            r.kind == kind::PLAN
                && r.text(field::PLAN_WALLET)
                    .and_then(|t| wallet::read_wallet(t).ok())
                    .is_some_and(|p| wallet::same_wallet(&p) == want)
        };
        // The notes on the map: those given, else those the record kept.
        let kept: Vec<String> = match notes {
            Some(n) => n.iter().map(plan::Note::to_text).collect(),
            None => open
                .contents
                .records
                .iter()
                .filter(|r| same(r))
                .flat_map(|r| {
                    r.fields
                        .iter()
                        .filter(|f| f.number == field::PLAN_NOTE)
                        .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
                        .collect::<Vec<_>>()
                })
                .collect(),
        };
        for n in &kept {
            record.push(field::PLAN_NOTE, n.as_bytes());
        }
        // Kept as it was: nothing to write back.
        let unchanged = open.contents.records.iter().any(|r| {
            same(r)
                && r.fields.len() == record.fields.len()
                && r.fields
                    .iter()
                    .zip(&record.fields)
                    .all(|(a, b)| a.number == b.number && *a.bytes == *b.bytes)
        });
        if unchanged {
            return;
        }
        let name = open.name.clone();
        let old: Vec<faraday_vault::records::Record> = open
            .contents
            .records
            .iter()
            .filter(|r| same(r))
            .cloned()
            .collect();
        if let Some(open) = self.vaults.open.get_mut(v) {
            open.contents.records.retain(|r| !same(r));
        }
        if !self.vault_push_in(v, record, &format!("The plan is in {name}")) {
            // It did not fit: the record it kept stays.
            if let Some(open) = self.vaults.open.get_mut(v) {
                open.contents.records.extend(old);
            }
        } else if !old.is_empty()
            && let Some(open) = self.vaults.open.get_mut(v)
        {
            open.changes += 1;
        }
    }

    fn backup_out(&mut self, what: u8) {
        let Some(wallet) = self.backup.as_ref().map(|b| b.wallet) else {
            return;
        };
        self.public_out(wallet, what);
    }

    /// The labelled pictures among a wallet's public files `what` (as
    /// [`Faraday::public_out`] numbers them): for 8, the descriptor's
    /// code, in parts past one code; for 4, each split share's code, one
    /// beside each share's sheet and text. None for any other file.
    pub fn public_pictures(&self, wallet: usize, what: u8) -> Vec<picture::Labelled> {
        let Some(w) = self.session.wallets.get(wallet) else {
            return Vec::new();
        };
        let stem = file_stem(&w.name);
        let net = self.session.network();
        let shape = if net.is_mainnet() {
            Session::shape(w)
        } else {
            format!("{} · {}", Session::shape(w), net.name())
        };
        let keys = w.policy.keys();
        let fps = |pick: &dyn Fn(usize) -> bool| -> Vec<String> {
            keys.iter()
                .enumerate()
                .filter(|(i, _)| pick(*i))
                .filter_map(|(_, k)| k.fingerprint().map(fp_text))
                .collect()
        };
        // Each code of `text` as a picture: `name` when there is one,
        // `name{part}j-of-k` for each part when there are more.
        let pictures = |name: &str, part: &str, text: &str, lines: Vec<String>| {
            let codes = picture_codes(text).unwrap_or_default();
            let k = codes.len();
            codes
                .into_iter()
                .enumerate()
                .map(|(j, code)| {
                    let (name, mut label) = if k == 1 {
                        (format!("{name}.png"), Vec::new())
                    } else {
                        (
                            format!("{name}{part}{}-of-{k}.png", j + 1),
                            vec![format!("Part {} of {k}", j + 1)],
                        )
                    };
                    label.extend(lines.iter().cloned());
                    picture::Labelled {
                        name,
                        code,
                        title: w.name.clone(),
                        lines: label,
                    }
                })
                .collect::<Vec<_>>()
        };
        match what {
            8 => {
                let text = w.policy.to_descriptor_checksummed();
                let all = fps(&|_| true);
                let mut lines = vec![shape];
                if !all.is_empty() {
                    let word = if all.len() == 1 { "Key" } else { "Keys" };
                    lines.push(format!("{word} {}", all.join(" · ")));
                }
                if let Some((_, sum)) = text.rsplit_once('#') {
                    lines.push(format!("Descriptor checksum {sum}"));
                }
                lines.push("Public: watch only, spends nothing".to_string());
                pictures(&format!("{stem}-descriptor"), "-", &text, lines)
            }
            4 => {
                let (m, n) = Session::quorum(w);
                // As the shares' sheets and text are split.
                let omit = self
                    .backup
                    .as_ref()
                    .map_or(m.saturating_sub(1), |b| b.answers.omit);
                let plan = backup::split_plan(n, m, omit);
                let mut out = Vec::new();
                for (i, row) in plan.iter().enumerate() {
                    let Some(text) = backup::split_share(w, row) else {
                        continue;
                    };
                    let mut lines = vec![
                        format!(
                            "Share {} of {} · any {m} rebuild the wallet",
                            i + 1,
                            plan.len()
                        ),
                        shape.clone(),
                    ];
                    let held = fps(&|k| row.contains(&k));
                    if !held.is_empty() {
                        lines.push(format!("Holds {}", held.join(" · ")));
                    }
                    let off = fps(&|k| !row.contains(&k));
                    if !off.is_empty() {
                        lines.push(format!("Leaves off {}", off.join(" · ")));
                    }
                    lines.push("Not a wallet on its own".to_string());
                    lines.push("Public: spends nothing".to_string());
                    out.extend(pictures(
                        &format!("{stem}-share-{}-of-{n}", i + 1),
                        "-part-",
                        &text,
                        lines,
                    ));
                }
                out
            }
            _ => Vec::new(),
        }
    }

    /// One of a wallet's public files to the Outbox: 0 the blank
    /// template, 1 the descriptor, 2 the multisig config, 3 the backup
    /// sheet, 4 the split shares, 5 the wallet .json, 6 the BIP 129
    /// descriptor record, 7 Bitcoin Core's `importdescriptors` file, 8 the
    /// descriptor's code as a labelled picture.
    pub(crate) fn public_out(&mut self, wallet: usize, what: u8) {
        let files = self.public_made(wallet, what);
        let names: Vec<String> = files.iter().map(|(n, _)| n.clone()).collect();
        for (name, bytes) in files {
            self.put_outbox(&name, bytes);
        }
        self.backup_sent(wallet, &names);
        match names.len() {
            0 => {}
            1 => self.toast_out(&names[0]),
            k => self.toast_out(&format!("{k} files")),
        }
    }

    /// Removes from For the stick what [`Faraday::public_out`] makes for
    /// `wallet` and `what`.
    fn public_remove(&mut self, wallet: usize, what: u8) {
        let names: Vec<String> = self
            .public_made(wallet, what)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        let before = self.outbox.len();
        self.outbox.retain(|i| !names.contains(&i.name));
        if self.outbox.len() != before {
            self.save_boxes();
        }
    }

    /// The backup under way's wallet, when its checklist is showing and
    /// file `what` is For the stick: that file is made again when what it
    /// depends on changes. The old one goes now.
    fn backup_remake(&mut self, what: u8) -> Option<usize> {
        let b = self.backup.as_ref()?;
        if b.stage != BStage::Checklist {
            return None;
        }
        let w = b.wallet;
        let made: Vec<String> = self
            .public_made(w, what)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        if !self.outbox.iter().any(|i| made.contains(&i.name)) {
            return None;
        }
        self.public_remove(w, what);
        Some(w)
    }

    /// Making the checklist makes every public file it calls for, For
    /// the stick: the blank template, the sheet or the shares, and the
    /// files for the software chosen. Made again, each replaces its
    /// namesake.
    fn backup_files_make(&mut self) {
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let w = b.wallet;
        let (qr, text) = (
            b.answers.form[plan::form::QR],
            b.answers.form[plan::form::TEXT],
        );
        let split = self
            .session
            .wallets
            .get(w)
            .is_some_and(|wl| b.answers.split && backup::splits(wl));
        let items: Vec<plan::Item> = self
            .backup_items()
            .into_iter()
            .filter_map(bstep::item)
            .collect();
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        for item in items {
            match item {
                plan::Item::Templates => files.extend(self.public_made(w, 0)),
                plan::Item::Sheets => files.extend(self.public_made(w, if split { 4 } else { 3 })),
                plan::Item::PublicFiles => {
                    // In the form chosen, as `backup_public_names` lists
                    // them: the picture, the text, or both.
                    for what in self.backup_public() {
                        let picture: Vec<(String, Vec<u8>)> = match what {
                            _ if !qr => Vec::new(),
                            1 => self.public_made(w, 8),
                            2 | 6 => {
                                let code = if what == 2 {
                                    Code::MultisigConfig(w)
                                } else {
                                    Code::Bsms(w)
                                };
                                self.code_view(code)
                                    .ok()
                                    .and_then(|v| v.png())
                                    .into_iter()
                                    .collect()
                            }
                            _ => Vec::new(),
                        };
                        if picture.is_empty() || text {
                            files.extend(self.public_made(w, what));
                        }
                        files.extend(picture);
                    }
                }
                _ => {}
            }
        }
        let names: Vec<String> = files.iter().map(|(n, _)| n.clone()).collect();
        for (name, bytes) in files {
            self.put_outbox(&name, bytes);
        }
        self.backup_sent(w, &names);
        match names.len() {
            0 => {}
            1 => self.toast_out(&names[0]),
            k => self.toast_out(&format!("{k} files")),
        }
    }

    /// The files [`Faraday::public_out`] makes, with their bytes.
    fn public_made(&mut self, wallet: usize, what: u8) -> Vec<(String, Vec<u8>)> {
        let Some(w) = self.session.wallets.get(wallet) else {
            return Vec::new();
        };
        let words = self.backup.as_ref().map_or(24, |b| b.words);
        let omit = self
            .backup
            .as_ref()
            .map_or(Session::quorum(w).0.saturating_sub(1), |b| b.answers.omit);
        let stem = file_stem(&w.name);
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        match what {
            // The sheets go out as PDFs, ready to print anywhere: they
            // hold nothing secret.
            0 => match pdf::sheet(&backup::sheet_blank(
                words,
                Some(w),
                self.session.network(),
                self.backup.as_ref().map_or(1, |b| b.answers.places),
            )) {
                Ok(p) => files.push((format!("blank-template-{words}-words.pdf"), p)),
                Err(e) => self.toast(&format!("No PDF: {e}")),
            },
            1 => {
                files.push((
                    format!("{stem}-descriptor.txt"),
                    format!("{}\n", w.policy.to_descriptor_checksummed()).into_bytes(),
                ));
                // A threshold wallet signs only with its record, which
                // lists every public share.
                if let Some(r) = w.policy.record() {
                    files.push((
                        format!("{stem}-record.txt"),
                        format!("{}\n", r.to_text()).into_bytes(),
                    ));
                }
            }
            5 => files.push((
                format!("{stem}-wallet.json"),
                backup::wallet_json(&w.name, &w.policy).into_bytes(),
            )),
            6 => {
                if let Some(record) = self.wallet_file_text(wallet, 6) {
                    files.push((format!("{stem}-bsms.txt"), record.into_bytes()));
                }
            }
            7 => files.push((
                format!("{stem}-bitcoin-core.json"),
                osk_bip::core_import::import_descriptors(
                    &w.policy.to_descriptor_checksummed(),
                    osk_bip::core_import::Rescan::Start,
                )
                .into_bytes(),
            )),
            2 => {
                if let Some(c) = self.wallet_file_text(wallet, 2) {
                    files.push((format!("{stem}-multisig-config.txt"), c.into_bytes()));
                }
            }
            3 => match pdf::sheet(&backup::sheet_wallet(&self.session, w)) {
                Ok(p) => files.push((format!("{stem}-backup.pdf"), p)),
                Err(e) => self.toast(&format!("No PDF: {e}")),
            },
            4 => {
                let (m, n) = Session::quorum(w);
                let plan = backup::split_plan(n, m, omit);
                let pictures = self.public_pictures(wallet, 4);
                for (i, row) in plan.iter().enumerate() {
                    if let Some(c) = backup::split_share(w, row) {
                        // The sheet to print beside each file.
                        let sheet = backup::sheet_share(w, i + 1, plan.len(), &c);
                        if let Ok(pdf) = pdf::sheet(&sheet) {
                            files.push((format!("{stem}-share-{}-of-{n}.pdf", i + 1), pdf));
                        }
                        files.push((format!("{stem}-share-{}-of-{n}.txt", i + 1), c.into_bytes()));
                        // And its code, to scan back.
                        let share = format!("{stem}-share-{}-of-{n}", i + 1);
                        for p in pictures.iter().filter(|p| {
                            p.name
                                .strip_prefix(share.as_str())
                                .is_some_and(|rest| rest == ".png" || rest.starts_with("-part-"))
                        }) {
                            files.push((p.name.clone(), p.png()));
                        }
                    }
                }
            }
            8 => {
                for p in self.public_pictures(wallet, 8) {
                    files.push((p.name.clone(), p.png()));
                }
            }
            _ => {}
        }
        files
    }

    /// Files of wallet `wallet` just put in the Outbox, for the summary
    /// of its backup when one is under way.
    fn backup_sent(&mut self, wallet: usize, names: &[String]) {
        if let Some(b) = self.backup.as_mut()
            && b.wallet == wallet
        {
            for n in names {
                if !b.sent.contains(n) {
                    b.sent.push(n.clone());
                }
            }
        }
    }

    /// The account key in slot `slot` of wallet `i`, when this session
    /// holds its seed: for the cosigners of a wallet of more than one
    /// key. `None` for a threshold or silent payments wallet, whose
    /// slots are not account keys.
    pub fn wallet_key(&self, i: usize, slot: u8) -> Result<WalletKey<'_>, String> {
        let w = self
            .session
            .wallets
            .get(i)
            .ok_or("That wallet is no longer loaded")?;
        if w.policy.record().is_some() || w.policy.silent().is_some() {
            return Err("This wallet's keys are not account keys".into());
        }
        let pk = w
            .policy
            .keys()
            .get(usize::from(slot))
            .ok_or("This wallet has no such key")?;
        let fp = pk.fingerprint().ok_or("That key names no master")?;
        let key = self
            .session
            .keys
            .iter()
            .find(|k| k.master.fingerprint() == fp)
            .ok_or("That key is not held here")?;
        let kind = create::NewKind::of(&w.policy);
        // Create's own account for its kind: what its BIP 129 record and
        // its account code are made at. A key elsewhere has neither.
        let standard = kind.filter(|k| {
            k.key_text(&key.master)
                .ok()
                .and_then(|t| osk_bip::policy::PolicyKey::parse(&t).ok())
                .is_some_and(|t| t.xpub() == pk.xpub() && t.path() == pk.path())
        });
        Ok(WalletKey {
            fp: fp_text(fp),
            text: pk.key_text(),
            kind: kind.map_or_else(|| Session::shape(w), |k| k.name().to_string()),
            standard,
            master: &key.master,
        })
    }

    /// The slots of wallet `i` whose seeds are held here, one a key, as
    /// its public files list them: none for a wallet of one key.
    pub fn wallet_keys_here(&self, i: usize) -> Vec<u8> {
        let Some(w) = self.session.wallets.get(i) else {
            return Vec::new();
        };
        let mut seen = Vec::new();
        let mut out = Vec::new();
        if self.session.slots(w).len() < 2 {
            return out;
        }
        for slot in 0..w.policy.keys().len().min(usize::from(u8::MAX)) {
            let slot = slot as u8;
            if let Ok(k) = self.wallet_key(i, slot)
                && !seen.contains(&k.fp)
            {
                seen.push(k.fp);
                out.push(slot);
            }
        }
        out
    }

    /// A wallet key's BIP 129 key record, signed by the key: for a `wsh`
    /// or `sh(wsh)` multisig key at Create's own account.
    fn wallet_key_record(&self, i: usize, slot: u8) -> Result<(String, String), String> {
        let k = self.wallet_key(i, slot)?;
        k.standard
            .and_then(|kind| kind.bsms_record(k.master, &format!("Faraday key {}", k.fp)))
            .map(|r| (k.fp.clone(), r))
            .ok_or_else(|| "BIP 129 covers wsh and sh(wsh) multisig keys".to_string())
    }

    fn wallet_key_out(&mut self, i: usize, slot: u8) {
        match self.wallet_key(i, slot) {
            Ok(k) => {
                let name = format!("xpub-{}.txt", k.fp);
                let bytes = format!("# Account xpub {}, {}\n{}\n", k.fp, k.kind, k.text);
                self.put_outbox(&name, bytes.into_bytes());
                self.backup_sent(i, std::slice::from_ref(&name));
                self.toast_out(&name);
            }
            Err(e) => self.toast(&e),
        }
    }

    fn wallet_key_bsms(&mut self, i: usize, slot: u8) {
        match self.wallet_key_record(i, slot) {
            Ok((fp, text)) => {
                let name = format!("xpub-{fp}-bsms.txt");
                self.put_outbox(&name, text.into_bytes());
                self.backup_sent(i, std::slice::from_ref(&name));
                self.toast_out(&name);
            }
            Err(e) => self.toast(&e),
        }
    }

    /// A creation that has keys chosen and is not made yet: Create comes
    /// back to it.
    pub fn create_unfinished(&self) -> bool {
        self.create.as_ref().is_some_and(|c| {
            c.built.is_none() && c.slots.iter().any(|s| *s != create::Source::Empty)
        })
    }

    /// Slots left for a cosigner's key, in the creation under way.
    pub fn create_waiting(&self) -> usize {
        self.create.as_ref().map_or(0, |c| {
            c.slots
                .iter()
                .filter(|s| **s == create::Source::Later)
                .count()
        })
    }

    /// The account key expression for a slot held here, with its
    /// fingerprint, for the cosigners.
    fn create_key_text(&self, slot: u8) -> Result<(String, String), String> {
        let c = self.create.as_ref().ok_or("nothing being created")?;
        let Some(create::Source::Here(fp)) = c.slots.get(slot as usize) else {
            return Err("That key is not held here".into());
        };
        let key = self
            .session
            .keys
            .iter()
            .find(|k| k.master.fingerprint().0 == *fp)
            .ok_or("a chosen key is no longer loaded")?;
        Ok((
            fp_text(osk_bip::keys::Fingerprint(*fp)),
            c.kind.key_text(&key.master)?,
        ))
    }

    /// A slot's key as `ur:crypto-account`, for a `wsh` multisig, the one
    /// script that type writes a cosigner's key under.
    fn create_key_account(&self, slot: u8) -> Option<String> {
        let c = self.create.as_ref()?;
        let Some(create::Source::Here(fp)) = c.slots.get(slot as usize) else {
            return None;
        };
        let key = self
            .session
            .keys
            .iter()
            .find(|k| k.master.fingerprint().0 == *fp)?;
        c.kind.account_ur(&key.master)
    }

    /// A slot's key as a signed BIP 129 key record, with its
    /// fingerprint: for a `wsh` or `sh(wsh)` multisig only.
    fn create_key_bsms(&self, slot: u8) -> Option<(String, String)> {
        let c = self.create.as_ref()?;
        let Some(create::Source::Here(fp)) = c.slots.get(slot as usize) else {
            return None;
        };
        let key = self
            .session
            .keys
            .iter()
            .find(|x| x.master.fingerprint().0 == *fp)?;
        let fpt = fp_text(osk_bip::keys::Fingerprint(*fp));
        c.kind
            .bsms_record(&key.master, &format!("Faraday key {fpt}"))
            .map(|r| (fpt, r))
    }

    /// The key texts of a creation's slots, or why they are not ready.
    pub fn create_keys(&self) -> Result<Vec<String>, String> {
        let c = self.create.as_ref().ok_or("nothing being created")?;
        if c.kind.threshold() {
            return self.create_shares().map(|keys| {
                keys.iter()
                    .map(|k| fp_text(k.master.fingerprint()))
                    .collect()
            });
        }
        let mut out = Vec::new();
        for (i, s) in c.slots.iter().enumerate() {
            match s {
                create::Source::Empty => return Err(format!("Key {} has no key yet", i + 1)),
                create::Source::Later => {
                    return Err(format!(
                        "Key {} waits for its cosigner's xpub file. Copy it in on {} visit, \
                         then choose it for Key {} under Keys",
                        i + 1,
                        self.medium.a(),
                        i + 1
                    ));
                }
                create::Source::Here(fp) => {
                    let key = self
                        .session
                        .keys
                        .iter()
                        .find(|k| k.master.fingerprint().0 == *fp)
                        .ok_or("a chosen key is no longer loaded")?;
                    out.push(c.kind.key_text(&key.master)?);
                }
                create::Source::Cosigner(text) => out.push(text.clone()),
            }
        }
        let mut seen = out.clone();
        seen.sort();
        seen.dedup();
        if seen.len() != out.len() {
            return Err("The same key is in two slots".into());
        }
        Ok(out)
    }

    /// The slot a cosigner's key goes in, while a multisig is being made:
    /// one marked for later, else the first with no key.
    fn create_waiting_slot(&self) -> Option<u8> {
        let c = self.create.as_ref()?;
        if c.built.is_some() || !c.kind.multi() || c.kind.threshold() {
            return None;
        }
        c.slots
            .iter()
            .position(|s| *s == create::Source::Later)
            .or_else(|| c.slots.iter().position(|s| *s == create::Source::Empty))
            .map(|i| i as u8)
    }

    /// The loaded keys a threshold creation deals from, in slot order:
    /// each must be 24 words, since 24 words are a share's 32 bytes.
    fn create_shares(&self) -> Result<Vec<&wallet::Key>, String> {
        let c = self.create.as_ref().ok_or("nothing being created")?;
        let mut out: Vec<&wallet::Key> = Vec::new();
        for (i, s) in c.slots.iter().enumerate() {
            let create::Source::Here(fp) = s else {
                return Err(format!("Share {} has no key yet", i + 1));
            };
            let key = self
                .session
                .keys
                .iter()
                .find(|k| k.master.fingerprint().0 == *fp)
                .ok_or("a chosen key is no longer loaded")?;
            if key.share.is_none() {
                return Err(format!("Share {} needs a 24-word key", i + 1));
            }
            if out.iter().any(|k| k.master.fingerprint().0 == *fp) {
                return Err("The same key is in two slots".into());
            }
            out.push(key);
        }
        Ok(out)
    }

    /// Deals a threshold wallet from the chosen keys: the other shares and
    /// the group key are computed, the record becomes the wallet, and each
    /// computed share is loaded as a key so its words can be written down
    /// in the backup. Locking forgets all of them.
    fn create_deal(&mut self) {
        let secp = osk_bip::bitcoin::secp256k1::Secp256k1::new();
        let (m, n) = match self.create.as_ref() {
            Some(c) => (c.m, c.n),
            None => return,
        };
        let dealt = self.create_shares().and_then(|keys| {
            let mut chosen = Vec::new();
            for (id, key) in keys.iter().enumerate() {
                let mut got = None;
                key.with_secret_share(&mut |s| {
                    let mut b = s.secret_bytes();
                    got = osk_bip::frost::SecShare::from_bytes(&b).ok();
                    zeroize::Zeroize::zeroize(&mut b);
                });
                chosen.push((id as u32, got.ok_or("a chosen key is not a share")?));
            }
            osk_bip::frost::deal(&secp, n, m, &chosen).map_err(|e| format!("{e:?}"))
        });
        let dealt = match dealt {
            Ok(d) => d,
            Err(e) => {
                if let Some(c) = self.create.as_mut() {
                    c.error = Some(e);
                }
                return;
            }
        };
        let record = osk_bip::threshold::ThresholdRecord::new(
            dealt.info.clone(),
            self.session.network().kind(),
        );
        let name = format!("New wallet {}", self.session.wallets.len() + 1);
        let i = match self
            .session
            .add_wallet(&name, &record.to_text(), "Dealt here")
        {
            Ok(i) => i,
            Err(e) => {
                if let Some(c) = self.create.as_mut() {
                    c.error = Some(e.text());
                }
                return;
            }
        };
        for (id, share) in dealt.shares.iter().enumerate().skip(m) {
            let mut bytes = share.secret_bytes();
            let words =
                osk_bip::bip39::Mnemonic::from_entropy(osk_bip::bip39::Language::English, &bytes)
                    .map(|mn| {
                        zeroize::Zeroizing::new(
                            mn.indices()
                                .iter()
                                .map(|w| osk_bip::bip39::Language::English.word(*w))
                                .collect::<Vec<_>>()
                                .join(" "),
                        )
                    });
            zeroize::Zeroize::zeroize(&mut bytes);
            if let Ok(words) = words {
                let label = format!("Share {} of {name}", id + 1);
                let _ = self.session.add_words(&words, &label, None);
            }
        }
        self.chart_work.moved_to(&self.session, i);
        self.wallet = i;
        if let Some(c) = self.create.as_mut() {
            c.built = Some(i);
            c.error = None;
            c.done[cstep::KEYS as usize] = true;
            c.open = Some(cstep::CHECK);
            c.scroll.follow = true;
        }
    }

    /// Restore's Kind: a row pressed. The quorum goes to 2 of 3 for a
    /// multisig, one key for a single key; a description read has chosen
    /// already.
    fn restore_kind(&mut self, i: u8) {
        let Some(&kind) = create::NewKind::ALL.get(usize::from(i)) else {
            return;
        };
        let Some(r) = self.restore.as_mut().filter(|r| !r.described) else {
            return;
        };
        let Some(s) = r.seeds.as_mut() else {
            return;
        };
        let was = s.kind;
        s.fix(kind);
        if was.multi() != kind.multi() || was.all_sign() != kind.all_sign() {
            s.set_n(if kind.multi() { 3 } else { 1 });
        }
        r.error = None;
    }

    /// The open slot's two passphrases agree. Says so when they do not.
    fn restore_pass_ok(&mut self) -> bool {
        let Some(r) = self.restore.as_mut() else {
            return false;
        };
        if r.pass.as_str() != r.pass2.as_str() {
            r.error = Some("The two passphrases differ".to_string());
            return false;
        }
        r.pass_focus = None;
        r.error = None;
        true
    }

    /// Why Restore does not take the seed whose key is `fp`, if it does
    /// not: with the description known, not one of its keys; from the
    /// seeds alone, no slot left.
    pub(crate) fn restore_refusal(&self, fp: osk_bip::keys::Fingerprint) -> Option<String> {
        let r = self.restore.as_ref()?;
        if r.described {
            let w = self.session.wallets.get(r.wallet?)?;
            // A threshold wallet's slots are shares, matched as they load.
            if w.policy.record().is_some() {
                return None;
            }
            let ours = self
                .session
                .slots(w)
                .iter()
                .any(|s| s.fingerprint == Some(fp));
            return (!ours).then(|| NOT_A_KEY.to_string());
        }
        let s = r.seeds.as_ref()?;
        (!s.keys.contains(&fp.0) && s.first_empty().is_none())
            .then(|| "Every slot is filled".to_string())
    }

    /// A seed for Restore's open slot, loaded: with the slot's passphrase
    /// when it has none of its own, matched to the description's keys or
    /// put in the slot (`docs/NEW-WALLET.md` §12.1).
    pub(crate) fn restore_took(&mut self, fp: osk_bip::keys::Fingerprint) {
        let fp = match self.restore_with_pass(fp) {
            Ok(fp) => fp,
            Err(e) => {
                if let Some(r) = self.restore.as_mut() {
                    r.error = Some(e);
                }
                return;
            }
        };
        if let Some(why) = self.restore_refusal(fp) {
            if let Some(r) = self.restore.as_mut() {
                r.error = Some(why);
            }
            return;
        }
        // Matched: its slot reads "Key k of the wallet" and is here.
        let place = self
            .restore
            .as_ref()
            .and_then(|r| r.wallet.filter(|_| r.described))
            .and_then(|i| self.session.wallets.get(i))
            .and_then(|w| {
                self.session
                    .slots(w)
                    .iter()
                    .position(|s| s.fingerprint == Some(fp))
            });
        let Some(r) = self.restore.as_mut() else {
            return;
        };
        if let Some(k) = place {
            r.absent.retain(|&a| usize::from(a) != k);
        } else if !r.described
            && let Some(s) = r.seeds.as_mut()
        {
            s.take(fp.0);
        }
        r.error = None;
        r.close_pass();
    }

    /// The key `fp` with the open slot's passphrase: itself when the slot
    /// has none or the key came with its own, else the key its words and
    /// that passphrase make, added once it is one Restore takes.
    fn restore_with_pass(
        &mut self,
        fp: osk_bip::keys::Fingerprint,
    ) -> Result<osk_bip::keys::Fingerprint, String> {
        let Some(r) = self.restore.as_ref() else {
            return Ok(fp);
        };
        if r.pass.as_str().is_empty() {
            return Ok(fp);
        }
        let Some(key) = self
            .session
            .keys
            .iter()
            .find(|k| k.master.fingerprint() == fp)
        else {
            return Ok(fp);
        };
        if key.passphrase.is_some() {
            return Ok(fp);
        }
        let Some(words) = key.words.clone() else {
            return Err("A passphrase goes with BIP-39 words only".to_string());
        };
        let label = format!("{} · passphrase", key.label);
        let mut pass = secret_text::room();
        pass.push_str(r.pass.as_str());
        let with = Session::default()
            .add_words_with(&words, &pass, "", None)
            .map_err(|e| e.text())?;
        if let Some(why) = self.restore_refusal(with) {
            return Err(why);
        }
        match self.session.add_words_with(&words, &pass, &label, None) {
            Ok(fp) => Ok(fp),
            Err(wallet::Refusal::Duplicate(_)) => Ok(with),
            Err(e) => Err(e.text()),
        }
    }

    /// A description or a share read by the camera while Restore waits
    /// for one: the wallet loads, or the share is taken. Returns whether
    /// it was.
    fn restore_arrived(&mut self, at: usize) -> bool {
        let Some((kind, name)) = self.inbox.get(at).map(|i| (i.kind, i.name.clone())) else {
            return false;
        };
        let Some(r) = self.restore.as_mut().filter(|r| !r.described) else {
            return false;
        };
        match kind {
            FileKind::Wallet => {
                self.act(Action::RUse(at));
                true
            }
            FileKind::Share => {
                if !r.shares.contains(&name) {
                    r.shares.push(name);
                }
                r.open = Some(rstep::DESCRIPTION);
                r.scroll.follow = true;
                let whole =
                    restore::merge(&self.restore_share_texts()).is_ok_and(|m| m.whole.is_some());
                if whole {
                    self.act(Action::RRebuild);
                }
                true
            }
            _ => false,
        }
    }

    /// Restore's Make the wallet with the description known: every slot
    /// has its seed or is Not here.
    pub(crate) fn restore_described_make(&mut self) {
        let Some(r) = self.restore.as_ref() else {
            return;
        };
        let Some(w) = r.wallet.and_then(|i| self.session.wallets.get(i)) else {
            return;
        };
        let open = self
            .session
            .slots(w)
            .iter()
            .enumerate()
            .filter(|(k, s)| s.held_by.is_none() && !r.absent.contains(&(*k as u8)))
            .count();
        let Some(r) = self.restore.as_mut() else {
            return;
        };
        if open > 0 {
            r.error = Some(format!(
                "{open} {} still empty",
                if open == 1 { "slot" } else { "slots" }
            ));
            return;
        }
        r.next(rstep::SEEDS);
        self.refresh_spend();
    }

    /// Keys typed while Restore's passphrase fields have focus. Returns
    /// whether they took the key.
    fn restore_pass_key(&mut self, key: KeyIn) -> bool {
        if self.screen != Screen::Restore || self.sheet.is_some() {
            return false;
        }
        let Some(r) = self.restore.as_mut() else {
            return false;
        };
        let Some(f) = r.pass_focus else {
            return false;
        };
        let field = if f == 0 { &mut r.pass } else { &mut r.pass2 };
        match key {
            // BIP-39 passphrases here are printable ASCII (`to_seed`).
            KeyIn::Char(ch) if (' '..='~').contains(&ch) => field.push(ch),
            KeyIn::Backspace => {
                field.pop();
            }
            KeyIn::Tab => r.pass_focus = Some(1 - f.min(1)),
            KeyIn::Escape | KeyIn::Enter => r.pass_focus = None,
            _ => return false,
        }
        r.error = None;
        true
    }

    fn restore_from_vault(&mut self, v: usize, r: usize) {
        use faraday_vault::records::{field, kind};
        let Some(open) = self.vaults.open.get(v) else {
            return;
        };
        let Some(rec) = open.contents.records.get(r) else {
            return;
        };
        let (Some(text), name) = (
            rec.text(field::WALLET).map(str::to_string),
            rec.text(field::WALLET_NAME).unwrap_or("Wallet").to_string(),
        ) else {
            return;
        };
        let source = open.name.clone();
        self.restore_wallet(&name, &text, &source);
        let Some(w) = self
            .restore
            .as_ref()
            .and_then(|s| s.wallet)
            .and_then(|i| self.session.wallets.get(i))
        else {
            return;
        };
        // The vault's keys for this wallet's slots load with it.
        let want: Vec<String> = self
            .session
            .slots(w)
            .iter()
            .filter_map(|s| s.fingerprint.map(fp_text))
            .collect();
        let open = &self.vaults.open[v];
        let keep: BTreeSet<usize> = open
            .contents
            .of(kind::KEY)
            .filter(|(_, k)| {
                vault_screens::key_fingerprint(self, k).is_some_and(|fp| want.contains(&fp))
            })
            .map(|(i, _)| i)
            .chain([r])
            .collect();
        let all: BTreeSet<usize> = (0..open.contents.records.len()).collect();
        let saved = std::mem::replace(
            &mut self.vaults.open[v].skip,
            all.difference(&keep).copied().collect(),
        );
        let (keys, _) = self.vault_load(v, false);
        self.vaults.open[v].skip = saved;
        self.screen = Screen::Restore;
        if keys > 0 {
            self.toast(&format!(
                "{keys} {} from {source}",
                if keys == 1 { "key" } else { "keys" }
            ));
        }
    }

    /// Whether the key in a Create slot can take a passphrase there: one
    /// loaded here from BIP-39 words without one, on a wallet that is not
    /// a threshold wallet (`docs/NEW-WALLET.md` §3.6).
    pub fn create_pass_offered(&self, slot: usize) -> bool {
        let Some(c) = self.create.as_ref() else {
            return false;
        };
        let Some(create::Source::Here(fp)) = c.slots.get(slot) else {
            return false;
        };
        !c.kind.threshold()
            && c.built.is_none()
            && self
                .session
                .keys
                .iter()
                .find(|k| k.master.fingerprint().0 == *fp)
                .is_some_and(|k| k.words.is_some() && k.passphrase.is_none())
    }

    /// Lock in under a Create slot: adds the key the slot's words and the
    /// passphrase typed make, and puts it in the slot in place of the
    /// first, which stays loaded (`docs/NEW-WALLET.md` §3.6).
    fn create_pass_lock(&mut self) {
        if !self.selftest_passed() {
            return;
        }
        let Some(slot) = self.create.as_ref().and_then(|c| c.pass_slot) else {
            return;
        };
        if !self.create_pass_offered(usize::from(slot)) {
            return;
        }
        let Some(c) = self.create.as_mut() else {
            return;
        };
        if c.pass.as_str() != c.pass2.as_str() {
            c.pass_note = Some("The two passphrases differ".to_string());
            return;
        }
        // Empty means none: the key stays as it is.
        if c.pass.as_str().is_empty() {
            c.close_pass();
            return;
        }
        let Some(create::Source::Here(first)) = c.slots.get(usize::from(slot)).cloned() else {
            return;
        };
        let mut pass = secret_text::room();
        pass.push_str(c.pass.as_str());
        let Some(key) = self
            .session
            .keys
            .iter()
            .find(|k| k.master.fingerprint().0 == first)
        else {
            return;
        };
        let Some(words) = key.words.clone() else {
            return;
        };
        let label = format!("{} · passphrase", key.label);
        let (added, new) = match self.session.add_words_with(&words, &pass, &label, None) {
            Ok(fp) => (Ok(fp), true),
            // Loaded already with this passphrase: that key is the one.
            Err(wallet::Refusal::Duplicate(_)) => (
                Session::default()
                    .add_words_with(&words, &pass, "", None)
                    .map_err(|e| e.text()),
                false,
            ),
            Err(e) => (Err(e.text()), false),
        };
        // The passphrase exists nowhere else: the key is made here
        // (`docs/NEW-WALLET.md` §14.3).
        if new && let Ok(fp) = &added {
            self.mark_made_here(*fp);
        }
        let Some(c) = self.create.as_mut() else {
            return;
        };
        match added {
            Ok(fp) => {
                if let Some(s) = c.slots.get_mut(usize::from(slot)) {
                    *s = create::Source::Here(fp.0);
                }
                if !c.pass_locked.contains(&fp.0) {
                    c.pass_locked.push(fp.0);
                }
                c.close_pass();
                self.refresh_spend();
            }
            Err(e) => c.pass_note = Some(e),
        }
    }

    /// Keys typed while a Create slot's passphrase fields have focus.
    /// Returns whether they took the key.
    fn create_pass_key(&mut self, key: KeyIn) -> bool {
        if self.screen != Screen::Create || self.sheet.is_some() {
            return false;
        }
        let Some(c) = self.create.as_mut() else {
            return false;
        };
        let (Some(_), Some(f)) = (c.pass_slot, c.pass_focus) else {
            return false;
        };
        let field = if f == 0 { &mut c.pass } else { &mut c.pass2 };
        match key {
            KeyIn::Char(ch) if (' '..='~').contains(&ch) => field.push(ch),
            KeyIn::Backspace => {
                field.pop();
            }
            KeyIn::Tab => c.pass_focus = Some(1 - f.min(1)),
            KeyIn::Escape => c.pass_focus = None,
            KeyIn::Enter => {
                self.create_pass_lock();
                return true;
            }
            _ => return false,
        }
        c.pass_note = None;
        true
    }

    fn create_make(&mut self) {
        if self.create.as_ref().is_some_and(|c| c.kind.threshold()) {
            self.create_deal();
            return;
        }
        let keys = match self.create_keys() {
            Ok(k) => k,
            Err(e) => {
                if let Some(c) = self.create.as_mut() {
                    c.error = Some(e);
                }
                return;
            }
        };
        let Some(c) = self.create.as_ref() else {
            return;
        };
        let descriptor = c.kind.descriptor(c.m, &keys);
        let name = format!("New wallet {}", self.session.wallets.len() + 1);
        match self.session.add_wallet(&name, &descriptor, "Made here") {
            Ok(i) => {
                self.chart_work.moved_to(&self.session, i);
                self.wallet = i;
                if let Some(c) = self.create.as_mut() {
                    c.built = Some(i);
                    c.error = None;
                    c.done[cstep::KEYS as usize] = true;
                    c.open = Some(cstep::CHECK);
                    c.scroll.follow = true;
                }
            }
            Err(e) => {
                if let Some(c) = self.create.as_mut() {
                    c.error = Some(e.text());
                }
            }
        }
    }

    /// The texts of the shares chosen on the restore screen.
    pub fn restore_share_texts(&self) -> Vec<String> {
        let Some(r) = self.restore.as_ref() else {
            return Vec::new();
        };
        self.inbox
            .iter()
            .filter(|i| r.shares.contains(&i.name))
            .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
            .collect()
    }

    /// Loads the wallet a description gives and fills Restore's Kind and
    /// Quorum from it: the seeds are matched to its keys next.
    fn restore_wallet(&mut self, name: &str, text: &str, source: &str) {
        match self.session.add_wallet(name, text, source) {
            Ok(i) => {
                self.wallet = i;
                let w = &self.session.wallets[i];
                let kind = create::NewKind::of(&w.policy);
                let (m, n) = Session::quorum(w);
                if let Some(r) = self.restore.as_mut() {
                    r.wallet = Some(i);
                    r.described = true;
                    r.absent.clear();
                    r.error = None;
                    if let Some(s) = r.seeds.as_mut() {
                        s.fix(kind.unwrap_or(s.kind));
                        s.keys.clear();
                        s.set_n(n);
                        s.set_m(m);
                    }
                    for k in [rstep::KIND, rstep::QUORUM, rstep::DESCRIPTION] {
                        r.done[k as usize] = true;
                    }
                    r.done[rstep::SEEDS as usize] = false;
                    r.done[rstep::CHECK as usize] = false;
                    r.open = Some(rstep::SEEDS);
                    r.scroll.follow = true;
                }
            }
            Err(e) => {
                if let Some(r) = self.restore.as_mut() {
                    r.error = Some(e.text());
                }
            }
        }
    }

    fn make_pdf(&mut self, name: &str, bytes: &[u8]) {
        if !self.online {
            return;
        }
        match pdf::sheet(&String::from_utf8_lossy(bytes)) {
            Ok(pdf) => self.storage_out.push_back(StorageCommand::Print {
                name: format!("{}.pdf", stem(name)),
                bytes: pdf,
            }),
            Err(e) => self.toast(&format!("Not printed: {e}")),
        }
    }

    /// One code the camera read: a part of a UR, a PSBT, or text.
    fn scanned(&mut self, bytes: Vec<u8>) {
        let bytes = zeroize::Zeroizing::new(bytes);
        let camera = self.sheet == Some(Sheet::Scan);
        let sticks = !self.sticks.is_empty();
        let medium = self.medium;
        let Some(scan) = self.scan.as_mut() else {
            return;
        };
        // Transfer's Receive takes anything and files nothing.
        if scan.purpose == ScanPurpose::Transfer {
            return self.transfer_scanned(bytes);
        }
        // A stick plugged in while the camera is on is held back, and no
        // key loads while it is attached: what loads one is not read.
        if camera && sticks && matches!(scan.purpose, ScanPurpose::Seed | ScanPurpose::KeyPart) {
            scan.note = Some(stick_keys(medium));
            return;
        }
        if scan.purpose == ScanPurpose::VaultEntry {
            let text = zeroize::Zeroizing::new(String::from_utf8_lossy(&bytes).into_owned());
            match self.vault_scanned_entry(&text) {
                Ok(()) => {
                    self.scan = None;
                    self.sheet = None;
                    self.commands.push_back(Command::CameraOff);
                }
                Err(why) => {
                    if let Some(scan) = self.scan.as_mut() {
                        scan.note = Some(why);
                    }
                }
            }
            return;
        }
        if scan.purpose == ScanPurpose::KeyPart {
            let text = match seedqr_words(&bytes) {
                Some(w) => w,
                None => zeroize::Zeroizing::new(String::from_utf8_lossy(&bytes).trim().to_string()),
            };
            self.entry.typed.set(&text);
            self.entry.error = None;
            self.form_add_part();
            match self.entry.error.clone() {
                None => {
                    self.scan = None;
                    self.sheet = None;
                    self.commands.push_back(Command::CameraOff);
                }
                Some(why) => {
                    zeroize::Zeroize::zeroize(&mut self.entry.typed);
                    if let Some(scan) = self.scan.as_mut() {
                        scan.note = Some(why);
                    }
                }
            }
            return;
        }
        if let ScanPurpose::CheckCopy(fp) = scan.purpose {
            self.copy_scanned(fp, bytes);
            return;
        }
        if scan.purpose == ScanPurpose::Seed {
            let mut bytes = bytes;
            let lang = osk_bip::bip39::Language::English;
            let digits = bytes.iter().all(u8::is_ascii_digit);
            let read = if digits {
                osk_codec::seedqr::from_digits(&bytes, lang)
            } else {
                osk_codec::seedqr::from_entropy(&bytes, lang)
            };
            zeroize::Zeroize::zeroize(&mut *bytes);
            match read {
                Ok(m) => {
                    let words = lang.words();
                    let mut typed = crate::secret_text::room();
                    for (k, &i) in m.indices().iter().enumerate() {
                        if k > 0 {
                            typed.push(' ');
                        }
                        typed.push_str(words[usize::from(i)]);
                    }
                    self.seed_scanned(&typed);
                }
                Err(_) => {
                    // Named, when it is something else.
                    let text = String::from_utf8_lossy(&bytes).to_string();
                    scan.note = Some(
                        what_instead_of_a_seed(&text)
                            .unwrap_or("Not a SeedQR or a CompactSeedQR")
                            .to_string(),
                    );
                }
            }
            return;
        }
        // Home's Scan takes a seed too: a SeedQR, a CompactSeedQR or the
        // words go to Add a key, as its own Scan does, and never into a
        // box (`docs/QR.md` §2).
        if camera && let Some(words) = seed_in_code(&bytes) {
            if self.may_load_keys() {
                self.act(Action::Entry(None));
                self.seed_scanned(&words);
                return;
            }
            if sticks {
                if let Some(scan) = self.scan.as_mut() {
                    scan.note = Some(stick_keys(medium));
                }
                return;
            }
        }
        let Some((named, ext, data)) = self.read_code(bytes) else {
            return;
        };
        self.scanned += 1;
        // A file in the Faraday file envelope is a file sent as one, of
        // whatever kind (`docs/QR.md` §2): it is filed under its name.
        let enveloped = named.is_some();
        let name = match named {
            Some(n) => self.free_inbox_name(&n),
            None => format!("scanned-{}.{ext}", self.scanned),
        };
        let item = Item::new(&name, data);
        if item.kind == FileKind::Other && !enveloped {
            if let Some(scan) = self.scan.as_mut() {
                scan.note =
                    Some("Not a PSBT, a descriptor, an xpub or a signed message".to_string());
                scan.decoder = osk_codec::ur::Decoder::new();
            }
            return;
        }
        let is_key = item.kind == FileKind::Key;
        self.inbox.push(item);
        self.save_boxes();
        self.scan = None;
        self.sheet = None;
        self.commands.push_back(Command::CameraOff);
        // A cosigner's account key scanned while a wallet is made from
        // seeds fills the box waiting for one.
        if is_key && self.seeds_take_file(self.inbox.len() - 1) {
            self.toast(&format!("{name} is a cosigner's key"));
            return;
        }
        // A cosigner's key scanned while a wallet is being made fills the
        // slot waiting for one.
        if is_key && let Some(slot) = self.create_waiting_slot() {
            let at = self.inbox.len() - 1;
            self.act(Action::CSlotFile(slot, at));
            self.toast(&format!("{name} is Key {}", slot + 1));
            self.screen = Screen::Create;
            return;
        }
        if self.screen == Screen::Family && self.family_arrived(self.inbox.len() - 1) {
            return;
        }
        if self.screen == Screen::Restore && self.restore_arrived(self.inbox.len() - 1) {
            return;
        }
        self.toast(&format!("{name} is in Files"));
        if !matches!(
            self.screen,
            Screen::Spend | Screen::Visit | Screen::Restore | Screen::Home | Screen::Family
        ) {
            self.screen = Screen::Files;
        }
    }

    /// A seed's words read by the camera, on Add a key: the camera closes
    /// and they are added as Add a key adds typed words.
    fn seed_scanned(&mut self, words: &str) {
        self.entry.typed.set(words);
        self.scan = None;
        self.sheet = None;
        self.commands.push_back(Command::CameraOff);
        // The words read are added as typed words, not through the word
        // keyboard, which a small panel gets back if they are refused.
        let keys = self.entry.keys.take();
        self.entry_add();
        if self.screen == Screen::Entry && keys.is_some() {
            self.entry.keys = Some(forms::word_typer(self.entry.language()));
        }
    }

    /// A seed's copy is checked once the numbers typed back match the seed
    /// on screen, as a scanned copy that matches makes it.
    fn backup_typed_check(&mut self) {
        let Some(b) = self.backup.as_mut() else {
            return;
        };
        let Some(key) = self.session.keys.get(b.key) else {
            return;
        };
        let Some(words) = key.words.as_ref() else {
            return;
        };
        let Ok(mn) = osk_bip::bip39::Mnemonic::parse(key.language, words.as_str()) else {
            return;
        };
        let digits = osk_codec::seedqr::to_digits(&mn);
        // A match is counted once: a copy more is typed after Clear.
        if !b.counted
            && backup::check_copy(&b.typed, digits.expose().as_bytes())
                == backup::CopyCheck::Matches
        {
            b.counted = true;
            let fp = key.master.fingerprint().0;
            self.copy_matched(fp);
        }
    }

    /// A code read while the backup's copy item scans the copy of the
    /// seed `fp`: decoded as a SeedQR and compared word by word with that
    /// seed, never loaded. A match closes the camera; a word that differs
    /// is named and the camera stays open for the copy fixed.
    fn copy_scanned(&mut self, fp: osk_bip::keys::Fingerprint, bytes: zeroize::Zeroizing<Vec<u8>>) {
        use osk_bip::bip39::Mnemonic;
        let copy = copy_indices(&bytes);
        drop(bytes);
        let Some(copy) = copy else {
            if let Some(scan) = self.scan.as_mut() {
                scan.note = Some("Not a SeedQR".to_string());
            }
            return;
        };
        let seed = self
            .session
            .keys
            .iter()
            .find(|k| k.master.fingerprint() == fp)
            .and_then(|k| {
                let words = k.words.as_ref()?;
                Mnemonic::parse(k.language, words.as_str()).ok()
            });
        let Some(seed) = seed else {
            // The seed is no longer here: nothing to compare with.
            self.scan = None;
            self.sheet = None;
            self.commands.push_back(Command::CameraOff);
            return;
        };
        let found = backup::compare_words(&copy, seed.indices());
        drop(copy);
        drop(seed);
        let matched = found == backup::CopyCheck::Matches;
        if matched {
            self.copy_matched(fp.0);
        }
        if let Some(b) = self.backup.as_mut() {
            b.scanned = Some(found.clone());
        }
        if matched {
            self.scan = None;
            self.sheet = None;
            self.commands.push_back(Command::CameraOff);
        } else if let Some(scan) = self.scan.as_mut() {
            scan.note = Some(backup::copy_scan_line(&found));
        }
    }

    /// What one code read as a file holds: its name when it carries one,
    /// its extension and its bytes. `None` while a transfer in parts waits
    /// for more, and for a code that is refused, which leaves its reason
    /// as the scan's note.
    /// What it read is wiped once done with: it may be a seed.
    fn read_code(
        &mut self,
        bytes: zeroize::Zeroizing<Vec<u8>>,
    ) -> Option<(Option<String>, &'static str, Vec<u8>)> {
        let scan = self.scan.as_mut()?;
        scan.reads += 1;
        let text = zeroize::Zeroizing::new(String::from_utf8_lossy(&bytes).trim().to_string());
        // A SeedQR is digits only; a seed never arrives by this door.
        if !text.is_empty()
            && text.bytes().all(|b| b.is_ascii_digit())
            && (text.len() == 48 || text.len() == 96)
        {
            scan.note =
                Some("That is a SeedQR. Seeds go in through Add a key, not Files".to_string());
            return None;
        }
        // A CompactSeedQR is 16 or 32 bytes of entropy, not text.
        if matches!(bytes.len(), 16 | 32) && std::str::from_utf8(&bytes).is_err() {
            scan.note = Some(
                "That may be a CompactSeedQR. Seeds go in through Add a key, not Files".to_string(),
            );
            return None;
        }
        // A wrong arrival is named, not refused generically (`docs/QR.md`
        // §1): a seed's words or a private key never go in a box.
        if let Some(why) = secret_text(&text) {
            scan.note = Some(why.to_string());
            return None;
        }
        // BBQr and numbered parts are put together here; anything else is
        // read as before.
        let mut item: Option<(Option<String>, &str, Vec<u8>)> = None;
        match scan.assembler.feed(&text) {
            faraday_qr::Step::Part {
                what,
                have,
                total,
                missing,
            } => {
                scan.note = Some(part_note(what, have, total, &missing));
                return None;
            }
            faraday_qr::Step::Refused(why) => {
                scan.note = Some(why);
                return None;
            }
            faraday_qr::Step::Done(arrived) => item = Some(arrival(arrived)),
            faraday_qr::Step::NotMine => {}
        }
        if item.is_none() && osk_codec::ur::is_ur(&text) {
            match scan.decoder.receive(&text) {
                Ok(true) => match scan.decoder.message() {
                    Some(osk_codec::ur::Message::Psbt(p)) => item = Some((None, "psbt", p.clone())),
                    Some(osk_codec::ur::Message::Bytes(b)) => {
                        item = Some(arrival(faraday_qr::classify_text(
                            &String::from_utf8_lossy(b),
                        )));
                    }
                    Some(osk_codec::ur::Message::Other { ur_type, cbor }) => {
                        match faraday_qr::registry::read(ur_type, cbor) {
                            Ok(Some(faraday_qr::registry::Read::Psbt(p))) => {
                                item = Some((None, "psbt", p));
                            }
                            Ok(Some(faraday_qr::registry::Read::Keys(t)))
                            | Ok(Some(faraday_qr::registry::Read::Descriptor(t))) => {
                                item = Some((None, "txt", t.into_bytes()));
                            }
                            Ok(None) => {
                                scan.note =
                                    Some(format!("ur:{ur_type} is not a kind this build reads"));
                            }
                            Err(why) => scan.note = Some(why),
                        }
                        if item.is_none() {
                            scan.decoder = osk_codec::ur::Decoder::new();
                            return None;
                        }
                    }
                    None => return None,
                },
                Ok(false) => {
                    if let Some((have, of)) = scan.decoder.progress() {
                        scan.note = Some(format!("UR: part {have} of {of}"));
                    }
                    return None;
                }
                Err(e) => {
                    scan.note = Some(format!("Not a part of this code: {e:?}"));
                    return None;
                }
            }
        }
        let (named, ext, data) = match item {
            Some(i) => i,
            None if wallet::read_psbt(&bytes).is_some() => (None, "psbt", bytes.to_vec()),
            None => {
                // An address is checked against the loaded wallets, not
                // filed.
                if let Some(said) = self.address_note(&text) {
                    if let Some(scan) = self.scan.as_mut() {
                        scan.note = Some(said);
                    }
                    return None;
                }
                arrival(faraday_qr::classify_text(&text))
            }
        };
        Some((named, ext, data))
    }

    /// What a scanned address is to the loaded wallets: one of a
    /// wallet's first hundred on either chain, or none of them (`docs/QR.md`
    /// §2: "Is this address mine?"). `None` when the text is not an
    /// address.
    fn address_note(&self, text: &str) -> Option<String> {
        let t = text.trim();
        let t = t.strip_prefix("bitcoin:").unwrap_or(t);
        let t = t.split('?').next().unwrap_or(t);
        let net = self.session.network();
        let Ok(addr) =
            t.parse::<osk_bip::bitcoin::Address<osk_bip::bitcoin::address::NetworkUnchecked>>()
        else {
            // An address's characters, as many as one has, that fail its
            // checksum: one of them is wrong.
            let whole = t.len() >= 26
                && osk_bip::keys::Network::ALL
                    .iter()
                    .any(|&n| osk_bip::address::is_address_prefix(t, n));
            return whole.then(|| {
                "That address fails its checksum: a character in it is wrong".to_string()
            });
        };
        let Ok(addr) = addr.require_network(net.into()) else {
            return Some(format!(
                "That address is not on {}",
                wallet::network_name(net)
            ));
        };
        for w in &self.session.wallets {
            if let Some((change, index)) = w.policy.find_address(net, &addr, 100) {
                let chain = if change { "change" } else { "receive" };
                return Some(format!(
                    "That address is {}'s {chain} address {index}",
                    w.name
                ));
            }
        }
        Some(if self.session.wallets.is_empty() {
            "That is an address. Load a wallet to check whether it is yours".to_string()
        } else {
            "That address is not among the first 100 of any loaded wallet".to_string()
        })
    }

    /// `name`, or `stem-2.ext`, … when the Inbox has it.
    fn free_inbox_name(&self, name: &str) -> String {
        let taken = |n: &str| self.inbox.iter().any(|i| i.name == n);
        if !taken(name) {
            return name.to_string();
        }
        let (stem, ext) = match name.rfind('.') {
            Some(i) if i > 0 => (&name[..i], &name[i..]),
            _ => (name, ""),
        };
        (2..1000)
            .map(|n| format!("{stem}-{n}{ext}"))
            .find(|n| !taken(n))
            .unwrap_or_else(|| name.to_string())
    }

    fn open_qr(&mut self, view: Result<QrView, String>) {
        match view {
            Ok(v) => {
                self.qr = Some(v);
                self.sheet = Some(Sheet::Qr);
            }
            Err(e) => self.toast(&format!("Cannot make a code: {e}")),
        }
    }

    /// A view's code as a labelled picture to the Outbox, when it is one
    /// code of public content.
    fn code_png(&mut self, view: Result<&QrView, &String>) {
        let v = match view {
            Ok(v) => v,
            Err(e) => return self.toast(&format!("Cannot make a code: {e}")),
        };
        match v.png() {
            Some((name, bytes)) => {
                self.put_outbox(&name, bytes);
                self.toast_out(&name);
            }
            None if v.public && v.frames.len() > 1 => {
                self.toast(&format!("{} is more than one code", v.title));
            }
            None => {}
        }
    }

    /// The QR view of a public file a flow makes: its code, and the name
    /// and label of its picture.
    pub fn code_view(&self, code: Code) -> Result<QrView, String> {
        let wallet = |i: usize| {
            self.session
                .wallets
                .get(i)
                .ok_or_else(|| "That wallet is no longer loaded".to_string())
        };
        match code {
            Code::Descriptor(i) => {
                let w = wallet(i)?;
                let stem = file_stem(&w.name);
                let mut lines = self.wallet_label(w);
                let (title, text, name) = match w.policy.record() {
                    Some(r) => (
                        format!("{} · threshold record", w.name),
                        r.to_text(),
                        format!("{stem}-record.png"),
                    ),
                    None => {
                        let text = w.policy.to_descriptor_checksummed();
                        if let Some((_, sum)) = text.rsplit_once('#') {
                            lines.push(format!("Descriptor checksum {sum}"));
                        }
                        (
                            format!("{} · wallet descriptor", w.name),
                            text,
                            format!("{stem}-descriptor.png"),
                        )
                    }
                };
                lines.push("Public: watch only, spends nothing".to_string());
                let mut view = QrView::text(&title, &text)?.public(&name, lines);
                view.whole = Some(text);
                Ok(view)
            }
            Code::MultisigConfig(i) | Code::Bsms(i) => {
                let w = wallet(i)?;
                let stem = file_stem(&w.name);
                let what = if code == Code::MultisigConfig(i) {
                    2
                } else {
                    6
                };
                let text = self
                    .wallet_file_text(i, what)
                    .ok_or("This wallet has no such file")?;
                let (title, name) = if what == 2 {
                    (
                        format!("{} · multisig config", w.name),
                        format!("{stem}-multisig-config.png"),
                    )
                } else {
                    (
                        format!("{} · BSMS descriptor record", w.name),
                        format!("{stem}-bsms.png"),
                    )
                };
                let mut lines = self.wallet_label(w);
                lines.push("Public: watch only, spends nothing".to_string());
                Ok(QrView::text(&title, &text)?.public(&name, lines))
            }
            Code::Key(k) => {
                let (fp, text) = self.create_key_text(k)?;
                let kind = self.create.as_ref().map_or("", |c| c.kind.name());
                let account = self.create_key_account(k);
                let lines = vec![
                    key_label(&fp, kind, &text),
                    "Public: spends nothing".to_string(),
                ];
                Ok(QrView::of(
                    &format!("Xpub {fp} · {kind}"),
                    QrSource::Key(text, account),
                    QrFormat::Ur,
                    QR_PARTS[1],
                )?
                .public(&format!("xpub-{fp}.png"), lines))
            }
            Code::KeyBsms(k) => {
                let (fp, key) = self.create_key_text(k)?;
                let kind = self.create.as_ref().map_or("", |c| c.kind.name());
                let (_, text) = self
                    .create_key_bsms(k)
                    .ok_or("BIP 129 covers wsh and sh(wsh) multisig keys")?;
                let lines = vec![
                    key_label(&fp, kind, &key),
                    "BIP 129 key record, signed by the key".to_string(),
                    "Public: spends nothing".to_string(),
                ];
                Ok(QrView::text(&format!("Key {fp} · BSMS record"), &text)?
                    .public(&format!("xpub-{fp}-bsms.png"), lines))
            }
            Code::WalletKey(i, slot) => {
                let k = self.wallet_key(i, slot)?;
                let account = k.standard.and_then(|kind| kind.account_ur(k.master));
                let lines = vec![
                    key_label(&k.fp, &k.kind, &k.text),
                    "Public: spends nothing".to_string(),
                ];
                Ok(QrView::of(
                    &format!("Xpub {} · {}", k.fp, k.kind),
                    QrSource::Key(k.text.clone(), account),
                    QrFormat::Ur,
                    QR_PARTS[1],
                )?
                .public(&format!("xpub-{}.png", k.fp), lines))
            }
            Code::CosignerKey(i, slot) => {
                let w = self
                    .session
                    .wallets
                    .get(i)
                    .ok_or("That wallet is no longer loaded")?;
                if w.policy.record().is_some() || w.policy.silent().is_some() {
                    return Err("This wallet's keys are not account keys".into());
                }
                let pk = w
                    .policy
                    .keys()
                    .get(usize::from(slot))
                    .ok_or("This wallet has no such key")?
                    .clone();
                let fp = pk
                    .fingerprint()
                    .map_or_else(|| "no origin".to_string(), fp_text);
                let text = pk.key_text();
                let lines = vec![
                    format!("Key {fp} · {}", Session::shape(w)),
                    "Public: spends nothing".to_string(),
                ];
                Ok(QrView::of(
                    &format!("Xpub {fp}"),
                    QrSource::Key(text, None),
                    QrFormat::Ur,
                    QR_PARTS[1],
                )?
                .public(&format!("xpub-{fp}.png"), lines))
            }
            Code::WalletKeyBsms(i, slot) => {
                let k = self.wallet_key(i, slot)?;
                let (_, text) = self.wallet_key_record(i, slot)?;
                let lines = vec![
                    key_label(&k.fp, &k.kind, &k.text),
                    "BIP 129 key record, signed by the key".to_string(),
                    "Public: spends nothing".to_string(),
                ];
                Ok(QrView::text(&format!("Key {} · BSMS record", k.fp), &text)?
                    .public(&format!("xpub-{}-bsms.png", k.fp), lines))
            }
            Code::Silent(_) | Code::SilentRecord => self.silent_code(code),
            Code::Message => {
                let m = self.message.as_ref().ok_or("No message is signed")?;
                let sig = m.signed.as_ref().ok_or("The message is not signed yet")?;
                let text = osk_psbt::message::signed_text(&sig.address, &sig.signature, &m.text);
                let name = format!(
                    "message-{}.png",
                    &sig.address[sig.address.len().saturating_sub(6)..]
                );
                let lines = vec![
                    format!("Signed by {}", sig.address),
                    "Public: proves who signed, spends nothing".to_string(),
                ];
                Ok(QrView::text("Signed message", &text)?.public(&name, lines))
            }
            Code::GpgKey | Code::GpgRevocation | Code::GpgSignature(_) => self.gpg_code(code),
        }
    }

    /// The lines that name a wallet under its code: its shape and
    /// network, and its keys' fingerprints.
    fn wallet_label(&self, w: &wallet::Wallet) -> Vec<String> {
        let net = self.session.network();
        let shape = if net.is_mainnet() {
            Session::shape(w)
        } else {
            format!("{} · {}", Session::shape(w), net.name())
        };
        let mut lines = vec![shape];
        let all: Vec<String> = w
            .policy
            .keys()
            .iter()
            .filter_map(|k| k.fingerprint().map(fp_text))
            .collect();
        if !all.is_empty() {
            let word = if all.len() == 1 { "Key" } else { "Keys" };
            lines.push(format!("{word} {}", all.join(" · ")));
        }
        lines
    }

    /// A wallet's public text file `what` (as [`Faraday::public_out`]
    /// numbers them): 2 the multisig config, 6 the BIP 129 record.
    fn wallet_file_text(&self, wallet: usize, what: u8) -> Option<String> {
        let w = self.session.wallets.get(wallet)?;
        match what {
            2 => backup::multisig_config(w, None),
            6 => {
                let net = self.session.network();
                let first = w.policy.address_at(net, false, 0).ok()?;
                Some(
                    osk_bip::bsms::DescriptorRecord {
                        policy: w.policy.clone(),
                        paths: vec!["/0/*".to_string(), "/1/*".to_string()],
                        first_address: first.to_string(),
                        network: net,
                    }
                    .to_text(),
                )
            }
            _ => None,
        }
    }

    /// Where leaving Add a key goes: the Spend tab when it came from
    /// there, else Wallets.
    pub(crate) fn entry_leave(&self) -> Screen {
        match self.entry.back {
            Some(s @ (Screen::Family | Screen::Spend)) => s,
            Some(Screen::Restore) if self.restore.as_ref().is_some_and(|r| r.seeds.is_some()) => {
                Screen::Restore
            }
            _ => Screen::Wallets,
        }
    }

    /// The word just typed into `entry.typed`, the way Tab completes it:
    /// the one word of the list this form's words come from that its
    /// letters can still become, if only one. `Codex32` has none, since
    /// it is typed as a string, not words.
    fn entry_tab_word(&self) -> Option<String> {
        match self.entry.form {
            forms::Form::Words => forms::completion(self.entry.language(), &self.entry.typed),
            forms::Form::Xor => {
                forms::completion(osk_bip::bip39::Language::English, &self.entry.typed)
            }
            forms::Form::Slip39 => forms::slip39_completion(&self.entry.typed),
            forms::Form::Codex32 => None,
        }
    }

    /// Completes the word just typed into `entry.typed`, the way Tab
    /// does for every form that types BIP-39 or SLIP-39 words one at a
    /// time.
    fn entry_complete_word(&mut self) {
        if let Some(word) = self.entry_tab_word() {
            let cut = self.entry.typed.rfind(' ').map(|i| i + 1).unwrap_or(0);
            self.entry.typed.truncate(cut);
            self.entry.typed.push_str(&word);
            self.entry.typed.push(' ');
        }
    }

    /// Whether the word just typed into `entry.typed` is already whole
    /// and the only one it can be: the point at which it moves on by
    /// itself, the same move Tab makes for you.
    fn entry_word_whole(&self) -> bool {
        let prefix = self.entry.typed.rsplit(' ').next().unwrap_or("");
        !prefix.is_empty() && self.entry_tab_word().as_deref() == Some(prefix)
    }

    fn entry_add(&mut self) {
        if !self.may_load_keys() {
            return;
        }
        let lang = self.entry.language();
        if let Some(w) = self.entry.keys.as_mut() {
            forms::take_typed(w, lang);
        }
        let wanted = self.entry.wanted.map(osk_bip::keys::Fingerprint);
        let label = self.entry.label.clone();
        let typed = match self.entry.keys.as_ref() {
            Some(w) => {
                let mut idx: Vec<u16> = w.committed_indices().collect();
                let m = osk_bip::bip39::Mnemonic::from_indices(self.entry.language(), &idx)
                    .map_err(|e| e.to_string());
                zeroize::Zeroize::zeroize(&mut idx);
                m
            }
            None => forms::typed_mnemonic(self.entry.language(), &self.entry.typed),
        };
        let m = match typed {
            Ok(m) => m,
            Err(e) => {
                self.entry.error = Some(e);
                return;
            }
        };
        // A SeedQR scanned over the Spend tab or Restore returns to it too.
        let back = self
            .entry
            .back
            .or(matches!(self.screen, Screen::Family | Screen::Restore).then_some(self.screen));
        // Restore takes a seed only into a slot it fills: with the
        // description known, one of its keys (`docs/NEW-WALLET.md` §12.1).
        if back == Some(Screen::Restore) {
            let fp = Session::default()
                .add_mnemonic(&m, &self.entry.passphrase, "", None)
                .ok();
            if let Some(why) = fp.and_then(|fp| self.restore_refusal(fp)) {
                if let Some(r) = self.restore.as_mut() {
                    r.error = Some(why);
                }
                self.entry = EntryState::default();
                self.screen = Screen::Restore;
                return;
            }
        }
        match self
            .session
            .add_mnemonic(&m, &self.entry.passphrase, &label, wanted)
        {
            Ok(fp) => {
                self.toast(&format!("Key {} added", fp_text(fp)));
                self.seeds_took(fp, back);
                self.entry = EntryState::default();
                self.refresh_spend();
                self.screen = match back {
                    Some(s) => s,
                    None if self.spend.is_some() => Screen::Spend,
                    None => Screen::Wallets,
                };
                if back == Some(Screen::Family) {
                    self.family_key_added();
                }
            }
            // A SeedQR scanned on Restore: the refusal shows there.
            Err(e) if self.screen == Screen::Restore => {
                if let Some(r) = self.restore.as_mut() {
                    r.error = Some(e.text());
                }
                self.entry = EntryState::default();
            }
            Err(e) => self.entry.error = Some(e.text()),
        }
    }

    fn load_wallet(&mut self, index: usize) {
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        let text = String::from_utf8_lossy(&item.bytes).into_owned();
        let name = stem(&item.name);
        let source = item.name.clone();
        let name = name
            .strip_suffix("-wallet")
            .unwrap_or(&name)
            .split(['-', '_'])
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ");
        // A wallet .json carries its own name.
        let name = wallet::json_string(&text, "label")
            .filter(|l| text.trim_start().starts_with('{') && !l.trim().is_empty())
            .unwrap_or(name);
        match self.session.add_wallet(&name, &text, &source) {
            Ok(i) => {
                self.wallet = i;
                self.screen = Screen::Wallets;
                self.refresh_spend();
            }
            Err(e) => self.toast(&e.text()),
        }
    }

    /// Loads the key an Inbox file's words spell, the same as typing them
    /// into Add a key. Refused with no stick attached.
    fn load_key(&mut self, index: usize) -> bool {
        if !self.may_load_keys() {
            return false;
        }
        let Some(item) = self.inbox.get(index) else {
            return false;
        };
        let Ok(text) = std::str::from_utf8(&item.bytes) else {
            return false;
        };
        let label = stem(&item.name);
        let words = forms::file_words(text);
        match self.session.add_words(&words, &label, None) {
            Ok(fp) => {
                self.toast(&format!("Key {} added", fp_text(fp)));
                self.screen = Screen::Wallets;
                self.refresh_spend();
                true
            }
            Err(e) => {
                self.toast(&e.text());
                false
            }
        }
    }

    /// Loads every key an "Import and load" copy asked for, now the stick
    /// that copy needed gone is gone. Returns how many loaded.
    fn load_after_pull(&mut self) -> usize {
        if self.visit.load_after.is_empty() {
            return 0;
        }
        let names = std::mem::take(&mut self.visit.load_after);
        let indices: Vec<usize> = self
            .inbox
            .iter()
            .enumerate()
            .filter(|(_, i)| names.contains(&i.name) && i.kind == FileKind::Words)
            .map(|(k, _)| k)
            .collect();
        let n = indices.into_iter().filter(|&i| self.load_key(i)).count();
        // A wallet in the Inbox that names a seed just loaded loads too.
        if n > 0 {
            self.inbox_load_named();
        }
        n
    }

    /// What a picture's codes held, read as the camera reads them.
    fn qr_read(&mut self, name: &str, payloads: Vec<Vec<u8>>) {
        if payloads.is_empty() {
            self.visit
                .log
                .push((format!("Copied {name}: no QR code read"), true));
        } else {
            // An image is read as the camera reads: a UR in parts can
            // span several images.
            if self.scan.is_none() {
                self.scan = Some(ScanState::default());
            }
            let before = self.inbox.len();
            let mut seeds = 0;
            for p in payloads {
                // A SeedQR on a stick is a seed's words in another
                // form: copied in as the words, and loaded when the
                // stick is removed, as Import and load does.
                if let Some(words) = seedqr_words(&p) {
                    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
                    let got = self.free_inbox_name(&format!("{stem}-words.txt"));
                    self.inbox.push(Item::new(&got, words.as_bytes().to_vec()));
                    self.visit.load_after.insert(got.clone());
                    self.save_boxes();
                    seeds += 1;
                    self.visit.log.push((
                        format!(
                            "{name}: a SeedQR, copied in as {got}; its key loads when the \
                                     {} is removed",
                            self.medium.noun()
                        ),
                        true,
                    ));
                    continue;
                }
                if self.scan.is_none() {
                    self.scan = Some(ScanState::default());
                }
                let parts = self.scan.as_ref().is_some_and(ScanState::in_parts);
                let n = self.inbox.len();
                self.scanned(p);
                // A transfer in parts that has just come together
                // ends the reading: what follows is another.
                if parts && self.inbox.len() > n {
                    break;
                }
            }
            let note = self.scan.as_ref().and_then(|s| s.note.clone());
            let seed_last = self
                .inbox
                .last()
                .is_some_and(|i| self.visit.load_after.contains(&i.name));
            if self.inbox.len() > before + seeds && !seed_last {
                // Named after the picture it came from.
                let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
                let taken: Vec<String> = self.inbox.iter().map(|i| i.name.clone()).collect();
                let mut got = String::new();
                if let Some(last) = self.inbox.last_mut() {
                    let ext = last
                        .name
                        .rsplit_once('.')
                        .map_or("txt", |(_, e)| e)
                        .to_string();
                    let wanted = format!("{stem}.{ext}");
                    if !taken.contains(&wanted) {
                        last.name = wanted;
                    }
                    got = last.name.clone();
                }
                self.save_boxes();
                self.toast(&format!("{got} is in Files"));
                self.visit
                    .log
                    .push((format!("{name} read into Files as {got}"), true));
            } else if seeds == 0 {
                self.visit.log.push((
                    format!("{name}: {}", note.unwrap_or_else(|| "read".into())),
                    true,
                ));
            }
        }
    }

    /// What a picture copied in holds, from what its codes added to the
    /// Inbox from `before` on: the kind most in need of care, a seed's
    /// words before any other secret, a secret before a sealed file, and
    /// [`FileKind::Other`] when they added nothing.
    fn picture_holds(&self, before: usize) -> FileKind {
        let care = |k: FileKind| match k.exposure() {
            _ if matches!(k, FileKind::Words | FileKind::SeedPart) => 5,
            secrets::Exposure::Secret => 4,
            _ if k.may_be_secret() => 3,
            secrets::Exposure::Sealed => 2,
            secrets::Exposure::Public => 1,
        };
        self.inbox
            .iter()
            .skip(before)
            .map(|i| i.kind)
            .max_by_key(|k| care(*k))
            .unwrap_or(FileKind::Other)
    }

    /// The Inbox files a stick visit offers to write, by index: every one
    /// but those the stick shown holds already, by name and size.
    pub fn visit_inbox_rows(&self) -> Vec<usize> {
        let on_stick = self.sticks.get(self.visit.stick).map(|s| &s.files);
        self.inbox
            .iter()
            .enumerate()
            .filter(|(_, i)| {
                !on_stick.is_some_and(|f| {
                    f.iter()
                        .any(|(n, len)| *n == i.name && *len == i.bytes.len() as u64)
                })
            })
            .map(|(k, _)| k)
            .collect()
    }

    /// The Inbox files Select all chooses on a visit: the public and
    /// sealed ones it lists, never one that may be a secret.
    fn visit_inbox_plain(&self) -> BTreeSet<String> {
        self.visit_inbox_rows()
            .into_iter()
            .filter_map(|k| self.inbox.get(k))
            .filter(|i| i.copy_ack().is_none())
            .map(|i| i.name.clone())
            .collect()
    }

    /// An Inbox file's row on a visit, pressed: unticked, or ticked when
    /// it is public or sealed. One that may be a secret goes to the secret
    /// sheet, and is ticked only once the person says they understand.
    fn visit_inbox(&mut self, i: usize) {
        if !self.visit_inbox_rows().contains(&i) {
            return;
        }
        let Some(item) = self.inbox.get(i) else {
            return;
        };
        let name = item.name.clone();
        if self.visit.from_inbox.remove(&name) {
            return;
        }
        let Some(ack) = item.copy_ack() else {
            self.visit.from_inbox.insert(name);
            return;
        };
        let holds = item.holds();
        let what = match (item.picture, holds) {
            (Some(FileKind::Other), _) => "A PNG with no QR code read".to_string(),
            (Some(k), _) => format!("A PNG of {}", screens::kind_name(k).to_lowercase()),
            (None, FileKind::Other) => "A file Faraday does not read".to_string(),
            (None, k) => screens::kind_name(k).to_string(),
        };
        let gives = match holds {
            FileKind::Words => "Whoever has it holds the key: spends its coins, or signs as it",
            FileKind::SeedPart => "Whoever has enough parts holds the key",
            FileKind::Entries => "Whoever has it reads every password and code in it",
            FileKind::Carry => "Whoever has it and a share's signature can work out that share",
            _ => "Whoever has it reads it",
        };
        // Text goes into a vault as a note; a picture or a file Faraday
        // does not read only to a stick.
        let keep = if item.picture.is_none()
            && matches!(
                holds,
                FileKind::Words | FileKind::SeedPart | FileKind::Entries | FileKind::Text
            ) {
            secrets::Keep::Note
        } else {
            secrets::Keep::None
        };
        let mut out = secrets::SecretOut::new(
            name,
            zeroize::Zeroizing::new(item.bytes.clone()),
            &what,
            gives,
        );
        out.keep = keep;
        out.ack = ack;
        out.to_visit = true;
        self.offer_secret(out);
    }

    /// Reads every chosen stick file into the Inbox.
    fn visit_copy(&mut self) {
        if let Some(stick) = self.sticks.get(self.visit.stick) {
            let id = stick.id.clone();
            self.visit.log.clear();
            for name in self.visit.inn.clone() {
                // A picture's QR codes are read, not its bytes.
                let stick = id.clone();
                self.storage_out
                    .push_back(if name.to_ascii_lowercase().ends_with(".png") {
                        StorageCommand::ReadQr { stick, name }
                    } else {
                        StorageCommand::Read { stick, name }
                    });
            }
        }
    }

    fn sign_here(&mut self) {
        // A key made here signs nothing until its backup is done
        // (`docs/NEW-WALLET.md` §14.3).
        let held = self.held_keys();
        let waiting: Vec<String> = self
            .spend
            .as_ref()
            .map(|s| {
                s.inspection
                    .participating_keys
                    .iter()
                    .filter(|fp| held.contains(fp))
                    .map(|fp| fp_text(*fp))
                    .collect()
            })
            .unwrap_or_default();
        if !waiting.is_empty() {
            if let Some(s) = self.spend.as_mut() {
                s.error = Some(format!(
                    "{} waits for its backup: finish the backup first",
                    waiting.join(", ")
                ));
                s.open = Some(wallet::step::SIGNERS);
                s.follow = true;
            }
            return;
        }
        // A MuSig2 or FROST round opened here draws its secret nonces
        // from a seed of this pass's own; a pass with no such input
        // draws nothing and needs none.
        let opens_round = self
            .spend
            .as_ref()
            .is_some_and(|s| spend_opens_round(&s.inspection));
        let seed = match self.sign_seed() {
            Some(seed) => seed,
            None if opens_round => {
                if let Some(s) = self.spend.as_mut() {
                    s.error = Some("No randomness from the system yet. Try again".to_string());
                }
                return;
            }
            None => [0u8; 32],
        };
        // The same transaction stating other amounts than when it was
        // signed here is refused, with no override (`docs/WALLETS.md` §3.3).
        if self
            .spend
            .as_ref()
            .is_some_and(|s| self.amounts_refused(&s.spend.psbt))
        {
            if let Some(s) = self.spend.as_mut() {
                s.error = Some(
                    "This transaction was signed here before with other amounts for its inputs. \
                     It is refused."
                        .to_string(),
                );
            }
            return;
        }
        let mut remember: Option<osk_psbt::Psbt> = None;
        let Some(s) = self.spend.as_mut() else {
            return;
        };
        let before = s.spend.psbt.to_bytes();
        // A threshold spend's first location chooses the other shares
        // before it signs.
        let needed = s.needed;
        if let Some(t) = s
            .inspection
            .inputs
            .first()
            .and_then(|i| i.threshold.as_ref())
            && t.signers.is_empty()
            && s.others.len() + 1 != needed
        {
            let want = needed.saturating_sub(1);
            s.error = Some(format!(
                "Choose {want} other {} under Signers",
                if want == 1 { "share" } else { "shares" }
            ));
            s.open = Some(wallet::step::SIGNERS);
            s.follow = true;
            return;
        }
        let mut psbt = s.spend.psbt.clone();
        let mut nonce_only = false;
        // A threshold pass signs as the shares held here; it names them
        // by their own fingerprints.
        let ours: Option<Vec<osk_bip::keys::Fingerprint>> = s
            .inspection
            .inputs
            .first()
            .and_then(|i| i.threshold.as_ref())
            .and_then(|t| {
                let record = self.session.wallets.get(s.wallet?)?.policy.record()?;
                Some(
                    t.ours
                        .iter()
                        .filter_map(|id| record.share_fingerprint(*id as usize))
                        .collect(),
                )
            });
        let signed =
            self.session
                .sign_with(&mut psbt, seed, &mut s.musig, &s.others, s.carry.as_ref());
        // A session whose every nonce has signed is over: the next pass
        // over this transaction opens a new one, never the same nonce.
        if s.musig.as_ref().is_some_and(|m| m.is_empty()) {
            s.musig = None;
        }
        let signed = signed.map(|out| {
            // A threshold pass that leaves shares to sign leaves their
            // nonce in a carry file; the one it read is spent.
            if out.carry.is_some() || s.carry.is_some() {
                s.round_spent |= s.carry.is_some();
                s.carry = None;
            }
            if let Some(section) = out.carry {
                s.carry_out = Some(zeroize::Zeroizing::new(osk_psbt::threshold::carry_bytes(
                    &section,
                    &psbt.to_bytes(),
                )));
            }
            ours.unwrap_or(out.by)
        });
        match signed {
            Ok(fps) if fps.is_empty() && s.musig.is_some() => {
                // MuSig2 round one: this device's nonce, and no signature.
                s.spend.psbt = psbt;
                s.error = None;
                nonce_only = true;
            }
            Ok(fps) => {
                remember = Some(psbt.clone());
                s.spend.psbt = psbt;
                s.spend.signed_here.extend(fps);
                s.done[5] = true;
                s.error = None;
            }
            Err(e) => s.error = Some(e),
        }
        if let Some(p) = remember {
            self.remember_amounts(&p);
            self.save_boxes();
        }
        if nonce_only {
            self.refresh_spend();
            self.part_out();
            if let Some(s) = self.spend.as_mut() {
                s.open = Some(wallet::step::NONCES);
                s.follow = true;
            }
            self.toast("This device's nonce is on the PSBT: carry it to the other signers");
            return;
        }
        self.refresh_spend();
        if self
            .spend
            .as_ref()
            .is_some_and(|s| s.spend.psbt.to_bytes() != before)
        {
            self.part_out();
        }
        if let Some(s) = self.spend.as_mut() {
            s.follow = true;
            if s.complete {
                s.done[wallet::step::COLLECT as usize] = true;
                s.open = Some(wallet::step::FINISH);
            } else if s.steps.contains(&wallet::step::COLLECT) {
                s.open = Some(wallet::step::COLLECT);
            } else {
                s.open = Some(wallet::step::FINISH);
            }
        }
    }

    fn collect(&mut self, index: usize) {
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        let bytes = item.bytes.clone();
        let name = item.name.clone();
        // Drawn before the spend is borrowed: a MuSig2 aggregation below
        // may still share a nonce, and never the one shared before.
        let seed = self.sign_seed();
        let Some(s) = self.spend.as_mut() else {
            return;
        };
        let before = s.spend.psbt.to_bytes();
        match s.spend.collect(&self.session, &bytes, &name) {
            Ok(added) if added.is_empty() && s.spend.psbt.to_bytes() == before => {
                s.error = Some(format!("{name} has nothing this copy lacks"));
            }
            Ok(_) => s.error = None,
            Err(e) => s.error = Some(e),
        }
        // A MuSig2 wallet: once every partial signature is in, this device
        // aggregates them; until then the pass changes nothing.
        let musig = self
            .session
            .wallet_for(&s.spend.psbt)
            .and_then(|i| self.session.wallets.get(i))
            .is_some_and(|w| wallet::Kind::of(&w.policy) == wallet::Kind::MuSig);
        if musig && !s.spend.signed_here.is_empty() && s.spend.finish().is_err() {
            let mut psbt = s.spend.psbt.clone();
            match seed {
                Some(seed) => {
                    if self.session.sign(&mut psbt, seed, &mut s.musig).is_ok() {
                        s.spend.psbt = psbt;
                    }
                    if s.musig.as_ref().is_some_and(|m| m.is_empty()) {
                        s.musig = None;
                    }
                }
                None => {
                    s.error = Some("No randomness from the system yet. Try again".to_string());
                }
            }
        }
        self.refresh_spend();
        if self
            .spend
            .as_ref()
            .is_some_and(|s| s.spend.psbt.to_bytes() != before)
        {
            self.part_out();
        }
        if let Some(s) = self.spend.as_mut() {
            s.follow = true;
            if s.complete {
                s.done[wallet::step::COLLECT as usize] = true;
                s.open = Some(wallet::step::FINISH);
            }
        }
    }

    fn primary(&mut self) {
        let Some(s) = self.spend.as_mut() else {
            return;
        };
        let complete = s.complete;
        let threshold = s
            .inspection
            .inputs
            .first()
            .and_then(|i| i.threshold.as_ref());
        let can_sign_here = match threshold {
            Some(t) => t.ours.iter().any(|id| !t.signed.contains(id)),
            None => s
                .inspection
                .participating_keys
                .iter()
                .any(|fp| self.session.key_label(*fp).is_some() && !s.signers.contains(&fp.0)),
        };
        let carry = threshold.is_some() && (s.carry_out.is_some() || s.out_signed) && !complete;
        if carry {
            if s.out_signed {
                self.screen = Screen::Files;
            } else {
                self.act(Action::CarryToOutbox);
            }
        } else if s.spend.finished.is_some() {
            // Both files went For the stick at Finish: Files writes them.
            self.screen = Screen::Files;
        } else if complete {
            s.done[wallet::step::COLLECT as usize] = true;
            s.open = Some(wallet::step::FINISH);
            s.follow = true;
        } else if can_sign_here {
            let before: Vec<u8> = s
                .steps
                .iter()
                .copied()
                .take_while(|&n| n != wallet::step::SIGN)
                .collect();
            if let Some(pos) = before.iter().position(|&n| !s.done[n as usize]) {
                s.open = Some(before[pos]);
                s.follow = true;
                s.error = Some(format!("Finish step {} first", pos + 1));
            } else {
                self.sign_here();
            }
        } else {
            s.open = Some(wallet::step::COLLECT);
            s.follow = true;
        }
    }

    /// Whether typing goes to a text field now.
    pub(crate) fn typing_field(&self) -> bool {
        if self.chart_typing() {
            return true;
        }
        if self.sheet == Some(Sheet::Import) {
            return self.vaults.focus == Some(vaults::Focus::Passphrase);
        }
        match self.screen {
            Screen::Family => self.vaults.focus.is_some() || self.seeds_typing(),
            Screen::Restore => {
                self.seeds_typing()
                    || self
                        .restore
                        .as_ref()
                        .is_some_and(|r| r.pass_focus.is_some())
            }
            Screen::Unlock | Screen::CreateVault | Screen::VaultContents => {
                self.vaults.focus.is_some()
            }
            Screen::Entry => true,
            Screen::Wallets => self.renaming.is_some(),
            Screen::Message => self.message.as_ref().is_some_and(|m| m.typing),
            Screen::KeyGen => self.keygen.as_ref().is_some_and(|k| {
                !k.locked
                    && ((k.focus.is_some() && k.open == Some(keygen::kstep::KEY))
                        || (k.open == Some(keygen::kstep::ENTER) && k.typing && k.flip_or_roll()))
            }),
            Screen::Create => self
                .create
                .as_ref()
                .is_some_and(|c| c.pass_slot.is_some() && c.pass_focus.is_some()),
            _ => false,
        }
    }

    /// Typing into the potential wallet's sheet. Returns whether the key
    /// was taken.
    fn potential_key(&mut self, key: KeyIn) -> bool {
        if self.sheet != Some(Sheet::Potential) {
            return false;
        }
        match key {
            KeyIn::Escape => {
                self.potential = None;
                self.sheet = None;
            }
            KeyIn::Enter => self.potential_make(),
            // Nothing secret is typed with a stick attached.
            _ if !self.sticks.is_empty() => {}
            KeyIn::Char(c) if !c.is_control() => {
                if let Some(p) = self.potential.as_mut()
                    && p.typing
                {
                    p.passphrase.push(c);
                    p.error = None;
                }
                self.potential_refresh();
            }
            KeyIn::Backspace => {
                if let Some(p) = self.potential.as_mut()
                    && p.typing
                {
                    p.passphrase.pop();
                    p.error = None;
                }
                self.potential_refresh();
            }
            _ => {}
        }
        true
    }

    fn key(&mut self, key: KeyIn) {
        if self.selftest_failed() {
            return;
        }
        if self.wordlist_key(key) {
            return;
        }
        if self.chart_key(key) {
            return;
        }
        if self.potential_key(key) {
            return;
        }
        if self.catalog_key(key) {
            return;
        }
        // The caret moved, or an edit inside the field: what is selected
        // goes with Backspace or Delete, and a character replaces it.
        if self.edit_key(key) {
            return;
        }
        // Delete in a field with no caret of its own takes the last
        // character, as Backspace does.
        let key = if key == KeyIn::Delete {
            KeyIn::Backspace
        } else {
            key
        };
        if self.seeds_key(key) {
            return;
        }
        if self.vault_key(key) {
            return;
        }
        if self.vanity_key_in(key) {
            return;
        }
        if self.tools_key(key) {
            return;
        }
        if self.lightning_key(key) {
            return;
        }
        if self.explore_key(key) {
            return;
        }
        if self.bip85_key(key) {
            return;
        }
        if self.keygen_key(key) {
            self.keygen_camera();
            self.keygen_device();
            return;
        }
        if self.create_pass_key(key) {
            return;
        }
        if self.restore_pass_key(key) {
            return;
        }
        if self.screen == Screen::Entry && self.entry.on_passphrase {
            match key {
                // BIP-39 passphrases here are printable ASCII (`to_seed`).
                KeyIn::Char(c) if (' '..='~').contains(&c) => self.entry.passphrase.push(c),
                KeyIn::Backspace => {
                    self.entry.passphrase.pop();
                }
                KeyIn::Enter if self.entry.form == forms::Form::Words => self.entry_add(),
                KeyIn::Enter => self.form_recover(),
                KeyIn::Escape | KeyIn::Tab => self.entry.on_passphrase = false,
                _ => {}
            }
            self.entry.error = None;
            return;
        }
        if self.screen == Screen::Entry && self.entry.form != forms::Form::Words {
            let codex32 = self.entry.form == forms::Form::Codex32;
            match key {
                KeyIn::Char(c) if c.is_ascii_alphabetic() || (codex32 && c.is_ascii_digit()) => {
                    self.entry.typed.push(c.to_ascii_lowercase());
                    self.entry.error = None;
                    if !codex32 && self.entry_word_whole() {
                        self.entry_complete_word();
                    }
                }
                KeyIn::Char(' ') if !codex32 => {
                    if !self.entry.typed.is_empty() && !self.entry.typed.ends_with(' ') {
                        self.entry.typed.push(' ');
                    }
                }
                KeyIn::Tab if !codex32 => self.entry_complete_word(),
                KeyIn::Backspace => {
                    self.entry.typed.pop();
                    self.entry.error = None;
                }
                KeyIn::Enter if self.entry.typed.trim().is_empty() => self.form_recover(),
                KeyIn::Enter => self.form_add_part(),
                KeyIn::Escape => self.screen = self.entry_leave(),
                _ => {}
            }
            return;
        }
        // A list with an on-screen keyboard takes what a physical one can
        // type of it: pinyin's letters and tone digits, or the list's
        // own characters from an input method.
        let lang = self.entry.language();
        if self.screen == Screen::Entry
            && let Some(w) = self.entry.keys.as_mut()
        {
            match key {
                KeyIn::Char(' ') | KeyIn::Tab => forms::take_typed(w, lang),
                KeyIn::Char(c) => {
                    w.type_char(c);
                    if forms::word_whole(w, lang) {
                        forms::take_typed(w, lang);
                    }
                }
                KeyIn::Backspace => w.backspace(),
                KeyIn::Enter => self.entry_add(),
                KeyIn::Escape => self.screen = self.entry_leave(),
                _ => {}
            }
            self.entry.error = None;
            return;
        }
        if self.screen == Screen::Entry {
            match key {
                KeyIn::Char(c) if c.is_ascii_alphabetic() => {
                    self.entry.typed.push(c.to_ascii_lowercase());
                    self.entry.error = None;
                    if self.entry_word_whole() {
                        self.entry_complete_word();
                    }
                }
                KeyIn::Char(' ') => {
                    if !self.entry.typed.is_empty() && !self.entry.typed.ends_with(' ') {
                        self.entry.typed.push(' ');
                    }
                }
                KeyIn::Backspace => {
                    self.entry.typed.pop();
                    self.entry.error = None;
                }
                KeyIn::Tab => self.entry_complete_word(),
                KeyIn::Enter => self.entry_add(),
                KeyIn::Escape => self.screen = self.entry_leave(),
                _ => {}
            }
            return;
        }
        if self.screen == Screen::Wallets
            && let Some(name) = self.renaming.as_mut()
        {
            match key {
                KeyIn::Char(c) if name.chars().count() < 40 => name.push(c),
                KeyIn::Backspace => {
                    name.pop();
                }
                KeyIn::Enter => {
                    let n = name.trim().to_string();
                    if !n.is_empty()
                        && let Some(w) = self.session.wallets.get_mut(self.wallet)
                    {
                        w.name = n;
                    }
                    self.renaming = None;
                }
                KeyIn::Escape => self.renaming = None,
                _ => {}
            }
            return;
        }
        if self.screen == Screen::Message
            && let Some(m) = self.message.as_mut()
            && m.typing
        {
            match key {
                KeyIn::Char(c) => {
                    m.text.push(c);
                    m.signed = None;
                }
                KeyIn::Enter => {
                    m.text.push('\n');
                    m.signed = None;
                }
                KeyIn::Backspace => {
                    m.text.pop();
                    m.signed = None;
                }
                KeyIn::Escape => m.typing = false,
                _ => {}
            }
            return;
        }
        if self.screen == Screen::Backup
            && let Some(b) = self.backup.as_mut()
        {
            match key {
                KeyIn::Char(c) if b.checking && c.is_ascii_digit() => {
                    b.typed.push(c);
                    self.backup_typed_check();
                    return;
                }
                KeyIn::Backspace if b.checking => {
                    b.typed.pop();
                    return;
                }
                KeyIn::Down if b.open.is_some_and(bstep::is_copy) => {
                    b.pin += 1;
                    return;
                }
                KeyIn::Up if b.open.is_some_and(bstep::is_copy) => {
                    b.pin = b.pin.saturating_sub(1);
                    return;
                }
                // A place's name, typed: kept in the vault alone.
                KeyIn::Char(c) if b.naming.is_some() => {
                    if let Some(name) = b.naming.and_then(|p| b.names.get_mut(p))
                        && name.chars().count() < 32
                        && !c.is_control()
                    {
                        name.push(c);
                    }
                    return;
                }
                KeyIn::Backspace if b.naming.is_some() => {
                    if let Some(name) = b.naming.and_then(|p| b.names.get_mut(p)) {
                        name.pop();
                    }
                    return;
                }
                KeyIn::Enter | KeyIn::Escape if b.naming.is_some() => {
                    b.naming = None;
                    return;
                }
                _ => {}
            }
        }
        match key {
            KeyIn::Escape => {
                // Escape is the sheet's Cancel: the scanner's camera goes
                // off with it, as on Cancel.
                if self.sheet.is_some() && self.sheet != Some(Sheet::Lock) {
                    self.act(Action::Cancel);
                }
            }
            KeyIn::Down => self.glide(60.0),
            KeyIn::Up => self.glide(-60.0),
            _ => {}
        }
    }

    /// The offset of the region that scrolls now, in design units: the
    /// open sheet's, the screen's, or another region of the screen's
    /// that the pointer went to ([`Faraday::slot`]).
    fn scroll_slot(&mut self) -> Option<&mut f32> {
        match self.slot() {
            ui::Slot::VisitOut => Some(&mut self.visit.out_offset),
            ui::Slot::Card => Some(&mut self.card_offset),
            ui::Slot::VaultItem => Some(&mut self.vault_item_offset),
            ui::Slot::Page => self.page_slot(),
        }
    }

    /// Which region scrolls now: the one the pointer last went to, while
    /// it can be reached; else the screen's or the sheet's own.
    fn slot(&self) -> ui::Slot {
        match self.region {
            ui::Slot::VisitOut
                if self.screen == Screen::Visit
                    && !self.compact
                    && self.region_key().1.is_none() =>
            {
                ui::Slot::VisitOut
            }
            ui::Slot::Card
                if self.screen == Screen::Wallets
                    && !self.compact
                    && self.region_key().1.is_none() =>
            {
                ui::Slot::Card
            }
            ui::Slot::VaultItem
                if self.screen == Screen::VaultContents
                    && !self.compact
                    && self.region_key().1.is_none() =>
            {
                ui::Slot::VaultItem
            }
            _ => ui::Slot::Page,
        }
    }

    /// The regions the last frame drew that can be reached now, last
    /// drawn first: through a sheet that does not scroll, none.
    fn regions(&self) -> std::vec::IntoIter<ui::Scrolled> {
        let key = self.region_key();
        let reachable = self.sheet.is_none() || key.1.is_some();
        let v: Vec<ui::Scrolled> = self
            .extent
            .iter()
            .filter(|(k, _)| reachable && *k == key)
            .flat_map(|(_, v)| v.iter().rev().copied())
            .filter(|e| e.slot == ui::Slot::Page || key.1.is_none())
            .collect();
        v.into_iter()
    }

    /// The region that scrolls now, as the last frame drew it.
    fn region_now(&self) -> Option<ui::Scrolled> {
        let slot = self.slot();
        let key = self.region_key();
        self.extent
            .iter()
            .filter(|(k, _)| *k == key)
            .flat_map(|(_, v)| v.iter().copied())
            .find(|e| e.slot == slot)
    }

    /// The wheel, a trackpad or a finger starts at (x, y): the region
    /// under it is the one they move, the page's when none is.
    fn point_region(&mut self, x: i32, y: i32) {
        self.region = self
            .regions()
            .find(|e| e.view.contains(x, y))
            .map_or(ui::Slot::Page, |e| e.slot);
    }

    /// The screen's region's offset, or the scrolling sheet's.
    fn page_slot(&mut self) -> Option<&mut f32> {
        match self.sheet {
            Some(Sheet::Learn) => return Some(&mut self.learn.scroll),
            Some(Sheet::WordList) => return self.wordlist.as_mut().map(|w| &mut w.scroll),
            Some(s) if self.compact || s == Sheet::Import => {
                if self.sheet_scroll.1 != Some(s) {
                    self.sheet_scroll = (0.0, Some(s));
                }
                return Some(&mut self.sheet_scroll.0);
            }
            _ => {}
        }
        Some(match self.screen {
            Screen::Visit
            | Screen::Files
            | Screen::Wallets
            | Screen::Start
            | Screen::Decode
            | Screen::Catalog
            | Screen::Transfer
            | Screen::Upgrade
            | Screen::Backups
            | Screen::Settings => &mut self.list_offset,
            Screen::Home
            | Screen::Entry
            | Screen::CheckMessage
            | Screen::Vaults
            | Screen::Unlock
            | Screen::VaultContents
            | Screen::Explore
            | Screen::Lightning
            | Screen::Tools
                if self.compact =>
            {
                &mut self.list_offset
            }
            Screen::Family => &mut self.family.scroll.y,
            Screen::Vanity => &mut self.vanity.as_mut()?.scroll.y,
            Screen::KeyGen => &mut self.keygen.as_mut()?.scroll.y,
            Screen::Bip85 => &mut self.bip85.as_mut()?.scroll.y,
            Screen::Silent => &mut self.silent.as_mut()?.scroll.y,
            Screen::CreateVault => &mut self.vaults.create.as_mut()?.scroll.y,
            Screen::Spend => &mut self.spend.as_mut()?.scroll,
            Screen::Backup => &mut self.backup.as_mut()?.scroll.y,
            Screen::Message => &mut self.message.as_mut()?.scroll.y,
            Screen::Create => &mut self.create.as_mut()?.scroll.y,
            Screen::Restore => &mut self.restore.as_mut()?.scroll.y,
            _ => return None,
        })
    }

    /// Which region scrolls now: the screen, or the sheet over it when
    /// that sheet scrolls. A sheet that does not scroll leaves the screen
    /// the region, so what moves on it carries on under the sheet.
    fn region_key(&self) -> ScreenKey {
        (
            self.screen,
            self.sheet.filter(|s| {
                self.compact || matches!(s, Sheet::Learn | Sheet::WordList | Sheet::Import)
            }),
        )
    }

    /// The motion belongs to the region that scrolls now: what moved
    /// another screen, sheet or region is stopped.
    fn own_motion(&mut self) {
        let key = (self.region_key(), self.slot());
        if self.motion_for != key {
            self.motion_for = key;
            self.motion.stop();
        }
    }

    /// Runs `go` on the motion and the region that scrolls now, with how
    /// far it goes as the last frame drew it. A motion left over from
    /// another screen or sheet is stopped first.
    fn with_region<R>(
        &mut self,
        go: impl FnOnce(&mut motion::Motion, &mut motion::Region) -> R,
    ) -> Option<R> {
        self.own_motion();
        let extent = self.region_now();
        let (f, h) = (self.f.max(0.1), self.h);
        let mut m = std::mem::take(&mut self.motion);
        m.rigid = self.reduce_motion;
        let out = self.scroll_slot().map(|offset| {
            let mut region = motion::Region {
                offset,
                max: extent.map(|e| e.max),
                view: extent.map_or(h, |e| e.view.h as f32 / f),
                f,
            };
            go(&mut m, &mut region)
        });
        self.motion = m;
        out
    }

    /// Finds what can be pressed under the pointer in the frame on
    /// screen. Returns whether that changed.
    fn find_hovered(&mut self) -> bool {
        let now = self.hover.and_then(|(x, y)| {
            self.hits
                .iter()
                .rev()
                .find(|(r, _)| r.contains(x, y))
                .map(|(_, a)| *a)
        });
        std::mem::replace(&mut self.hovered, now) != now
    }

    /// Fingers moved the content by `dy` units: at once.
    fn pan(&mut self, dy: f32) {
        self.with_region(|m, r| m.pan(r, dy));
    }

    /// A wheel or a key asked for `dy` units: the content glides there.
    fn glide(&mut self, dy: f32) {
        if self.reduce_motion {
            self.with_region(|m, r| m.jump(r, dy));
        } else if self.scroll_slot().is_some() {
            // A glide on the region the pointer has just moved to starts
            // there, not on the region that was moving.
            self.own_motion();
            self.motion.glide(dy);
        }
    }

    /// Moves whatever is moving on to `now_ms`. Returns whether a frame
    /// is wanted for it.
    fn motion_tick(&mut self, now_ms: u64) -> bool {
        match self.with_region(|m, r| m.tick(r, now_ms)) {
            Some(moved) => moved,
            None => {
                self.motion.stop();
                false
            }
        }
    }

    fn touch(&mut self, x: u16, y: u16, phase: TouchPhase) {
        let (x, y) = (i32::from(x), i32::from(y));
        let found = self
            .hits
            .iter()
            .rev()
            .find(|(r, _)| r.contains(x, y))
            .map(|(r, a)| (*a, *r));
        match phase {
            TouchPhase::Down => {
                self.down_at = (x, y);
                self.point_region(x, y);
                // At the scrolled region's right edge a press takes the
                // scrollbar: held on its thumb it follows from where it was
                // taken, anywhere else on the track the thumb's middle
                // comes to the press.
                if let Some((e, g)) = self.bar_at(x, y) {
                    let offset = self.scroll_slot().map_or(0.0, |o| *o);
                    let at = g.top
                        + ((g.track - g.thumb) as f32 * (offset / e.max).clamp(0.0, 1.0)).round()
                            as i32;
                    let grab = if (at..at + g.thumb).contains(&y) {
                        y - at
                    } else {
                        g.thumb / 2
                    };
                    self.bar_held = Some(grab);
                    self.motion.stop();
                    self.pressed = None;
                    self.drag = None;
                    self.swallow = false;
                    self.bar_drag(y);
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                    return;
                }
                // A finger on a scrolled region may be the start of a
                // drag; it is a press until it moves. Not through a sheet
                // that does not scroll: the page under it is out of reach.
                let slot = self.slot();
                self.drag = self
                    .regions()
                    .find(|e| e.slot == slot && e.view.contains(x, y))
                    .map(|_| (y, false));
                // A touch on content that is still flying only stops it.
                self.swallow = self.motion.flung(self.f);
                if self.swallow {
                    self.motion.stop();
                    self.pressed = None;
                    return;
                }
                self.pressed = found;
                self.vaults.pressed_at = self.now_ms;
                if let Some((Action::VisitBar(c), _)) = found {
                    self.visit_drag(c, y);
                }
            }
            TouchPhase::Move => {
                if self.bar_held.is_some() {
                    self.bar_drag(y);
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                    return;
                }
                let held_bar = match self.pressed {
                    Some((Action::VisitBar(c), _)) => Some(c),
                    _ => None,
                };
                // A slider held follows the finger along its track.
                if let Some((Action::Slide(id, _), r)) = self.pressed {
                    let row = r.y + r.h / 2;
                    let under = self
                        .hits
                        .iter()
                        .rev()
                        .find(|(h, a)| {
                            matches!(a, Action::Slide(i, _) if *i == id) && h.contains(x, row)
                        })
                        .map(|(h, a)| (*a, *h));
                    if let Some((a, h)) = under
                        && Some(a) != self.pressed.map(|p| p.0)
                    {
                        self.pressed = Some((a, h));
                        self.act(a);
                        self.dirty = true;
                        self.commands.push_back(Command::Draw);
                    }
                    return;
                }
                if let Some((last, dragging)) = self.drag
                    && held_bar.is_none()
                {
                    let slop = motion::SLOP * self.f;
                    let (dx, dy) = (x - self.down_at.0, y - self.down_at.1);
                    let start = !dragging && (dy as f32).abs() > slop && dy.abs() > dx.abs();
                    if dragging || start {
                        // Past the slop the press is a drag: nothing
                        // under the finger is pressed, the page follows.
                        self.pressed = None;
                        self.drag = Some((y, true));
                        let from = if start { self.down_at.1 } else { last };
                        self.pan((from - y) as f32 / self.f.max(0.1));
                        self.dirty = true;
                        self.commands.push_back(Command::Draw);
                        return;
                    }
                }
                // The scrollbar follows the finger anywhere once held.
                if let Some(c) = held_bar {
                    self.visit_drag(c, y);
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                } else if let Some((_, r)) = self.pressed
                    && !r.contains(x, y)
                {
                    self.pressed = None;
                }
            }
            TouchPhase::Up => {
                if self.bar_held.take().is_some() {
                    return;
                }
                let dragged = self.drag.take().is_some_and(|(_, d)| d);
                if dragged {
                    self.pressed = None;
                    self.motion.release(self.f);
                }
                if dragged || std::mem::take(&mut self.swallow) {
                    return;
                }
                let pressed = self.pressed.take();
                // A hold acts on the tick that completes it, never on release.
                if let (Some((a, _)), Some((b, _))) = (pressed, found)
                    && a == b
                    && !vaults::is_hold(a)
                {
                    // On a field: the caret where it was pressed, a
                    // second press selecting all of it, a drag what it
                    // went over (`docs/NEW-WALLET.md` §13.2).
                    let double = self
                        .last_tap
                        .is_some_and(|(l, t)| l == a && self.now_ms.saturating_sub(t) <= 450);
                    if field_action(a)
                        && let Some(dragged) = self.field_press(a, self.down_at.0, x, double)
                    {
                        self.last_tap = (!double && !dragged).then_some((a, self.now_ms));
                        return;
                    }
                    self.last_tap = Some((a, self.now_ms));
                    self.act(a);
                    self.keygen_camera();
                    self.keygen_device();
                    if field_action(a) {
                        self.field_press(a, x, x, false);
                    }
                }
            }
        }
    }

    /// The overlay scrollbar a press at (x, y) would take: the scrolled
    /// region's, when the press is within [`ui::BAR_GRAB`] of its right
    /// edge and the region scrolls, with where its bar runs. A small
    /// panel draws none, so a press there is on the content.
    fn bar_at(&self, x: i32, y: i32) -> Option<(ui::Scrolled, ui::BarGeometry)> {
        if self.compact {
            return None;
        }
        // Not through a sheet that does not scroll.
        let e = self.regions().find(|e| e.view.contains(x, y))?;
        if e.own_bar || e.max <= 0.0 {
            return None;
        }
        let f = self.f.max(0.1);
        if x < e.view.right() - (ui::BAR_GRAB * f).round() as i32 {
            return None;
        }
        let g = ui::BarGeometry::of(e.view, e.max, f);
        (g.track > g.thumb).then_some((e, g))
    }

    /// The scrolled region moved to where its held scrollbar is, at pixel
    /// row `y`.
    fn bar_drag(&mut self, y: i32) {
        let Some(grab) = self.bar_held else {
            return;
        };
        let Some(e) = self.region_now() else {
            return;
        };
        let g = ui::BarGeometry::of(e.view, e.max, self.f.max(0.1));
        let room = (g.track - g.thumb).max(1) as f32;
        let at = ((y - g.top - grab) as f32).clamp(0.0, room);
        if let Some(o) = self.scroll_slot() {
            *o = at / room * e.max;
        }
    }

    /// A stick visit column's list scrolled to where its scrollbar is
    /// held, at pixel row `y`: the thumb's middle under the finger.
    fn visit_drag(&mut self, column: Column, y: i32) {
        let Some((top, h, thumb, max)) = self.visit.bar(column).get() else {
            return;
        };
        let room = (h - thumb).max(1) as f32;
        let at = ((y - top) as f32 - thumb as f32 / 2.0).clamp(0.0, room);
        let offset = at / room * max;
        match column {
            Column::Outbox => self.visit.out_offset = offset,
            Column::Stick => self.list_offset = offset,
        }
    }

    fn render(&mut self) {
        let Some(mut canvas) = self.canvas.take() else {
            return;
        };
        // The online app reached mainnet some other way (a mainnet wallet
        // or transaction loaded): the warning comes up over it.
        if self.online
            && self.session.network().is_mainnet()
            && !self.airgap_warned
            && self.sheet.is_none()
        {
            self.sheet = Some(Sheet::NotAirgapped);
        }
        let mut hits = std::mem::take(&mut self.hits);
        hits.clear();
        // Offsets in whole pixels, whoever set them last, so a scrolled
        // page moves as one.
        let f = self.f.max(0.1);
        if let Some(offset) = self.scroll_slot() {
            *offset = (*offset * f).round() / f;
        }
        let stretch = (self.motion.stretch() * f).round() as i32;
        // The overlay scrollbar shows while the offset moves and fades
        // once it has rested a moment.
        let key = self.region_key();
        let slot = self.slot();
        let offset = self.scroll_slot().map_or(0.0, |o| *o);
        let page_offset = self.page_slot().map_or(0.0, |o| *o);
        let (seen_key, seen_offset, moved_at) = self.bar_seen;
        let moved_at = if seen_key != (key, slot) {
            0
        } else if seen_offset != offset || stretch != 0 {
            self.now_ms.max(1)
        } else {
            moved_at
        };
        // Held, or with the pointer over it, it stays in full.
        let hot =
            self.bar_held.is_some() || self.hover.is_some_and(|(x, y)| self.bar_at(x, y).is_some());
        let moved_at = if hot { self.now_ms.max(1) } else { moved_at };
        self.bar_seen = ((key, slot), offset, moved_at);
        let bar = motion::bar_alpha(moved_at, self.now_ms);
        // Drawn twice when a step card has just opened: the first time
        // finds out, the second draws it growing from closed.
        let mut follow_to = None;
        let mut scrolled = Vec::new();
        let mut caret = None;
        for _ in 0..2 {
            hits.clear();
            let disclosure = self
                .disclosed
                .as_ref()
                .filter(|d| d.key == key && d.moving)
                .map(|d| ui::Disclosure {
                    open: d.open,
                    closing: d.closing,
                    shown: motion::ease_out(motion::progress(
                        d.at,
                        self.now_ms,
                        motion::DISCLOSE_MS,
                    )),
                });
            let column = {
                let mut ui = ui::Ui::new(&mut canvas, self.f, &mut hits, self.pressed.map(|p| p.0));
                ui.edit = self.edit_now();
                ui.theme = self.theme;
                ui.hovered = self.hovered;
                ui.stretch = stretch;
                ui.bar = (bar > 0).then_some((bar, offset));
                ui.stretch_in_sheet = matches!(
                    self.sheet,
                    Some(Sheet::Learn | Sheet::WordList | Sheet::Import)
                ) || (self.compact && self.sheet.is_some());
                ui.disclosure = disclosure;
                let frost_key = ((self.screen, self.sheet), self.theme);
                ui.frost = self
                    .frost
                    .take()
                    .filter(|(k, _)| *k == frost_key)
                    .map(|(_, page)| page);
                ui.guided_shown = self.guided_shown();
                ui.about_open = self
                    .about_open
                    .filter(|(s, _)| *s == self.screen && self.sheet.is_none())
                    .map(|(_, k)| k);
                ui.offset = page_offset;
                ui.active = slot;
                ui.compact = self.compact;
                ui.caret_on = self.caret_on();
                screens::draw(self, &mut ui);
                caret = ui.caret_drawn.then_some(ui.caret_on);
                self.fields = std::mem::take(&mut ui.fields);
                self.back_link = ui.back_link;
                scrolled = std::mem::take(&mut ui.scrolled);
                follow_to = follow_to.or(ui.follow_to);
                self.frost = ui.frost.take().map(|page| (frost_key, page));
                ui.column
            };
            if !self.disclose(key, column) {
                break;
            }
        }
        if let Some(want) = follow_to {
            let now = self.scroll_slot().map_or(want, |o| *o);
            if self.reduce_motion {
                self.with_region(|m, r| m.jump(r, want - now));
            } else {
                self.motion.glide_to(want - now);
            }
            self.dirty = true;
            self.commands.push_back(Command::Draw);
        }
        self.extent = (!scrolled.is_empty()).then(|| (self.region_key(), scrolled));
        self.caret_drawn = caret;
        self.vaults.drawn = true;
        self.hits = hits;
        self.canvas = Some(canvas);
        // The frame moved things under a still pointer: the next frame
        // shows what it is over now.
        if self.find_hovered() {
            self.dirty = true;
            self.commands.push_back(Command::Draw);
        }
    }

    /// Whether the text caret shows now: it blinks, on for a half period
    /// and off for one, starting on at the last key or press. Steady with
    /// reduce motion.
    fn caret_on(&self) -> bool {
        self.reduce_motion
            || (self.now_ms.saturating_sub(self.caret_at) / motion::CARET_HALF_MS).is_multiple_of(2)
    }

    /// Where the Guided switch's pill shows: 0 on Steps only, 1 on
    /// Guided, between while it slides.
    fn guided_shown(&self) -> f32 {
        let to = if self.guided { 1.0 } else { 0.0 };
        match self.guided_moving {
            Some((from, at)) => {
                let t = motion::ease_out(motion::progress(at, self.now_ms, motion::SWITCH_MS));
                from + (to - from) * t
            }
            None => to,
        }
    }

    /// Takes what the step column drew: which card is open, and its
    /// body's height. Returns whether a card has just opened, so the
    /// frame is drawn again with it growing.
    fn disclose(&mut self, key: ScreenKey, column: Option<(Option<usize>, f32)>) -> bool {
        let Some((open, body_h)) = column else {
            self.disclosed = None;
            return false;
        };
        let opened = match self.disclosed.as_mut() {
            Some(d) if d.key == key && d.open != open && self.reduce_motion => {
                d.open = open;
                d.body_h = body_h;
                false
            }
            Some(d) if d.key == key && d.open != open => {
                let closing = d.open.map(|i| (i, d.body_h));
                *d = Disclosed {
                    key,
                    open,
                    body_h,
                    closing,
                    at: None,
                    moving: true,
                };
                true
            }
            Some(d) if d.key == key => {
                d.body_h = body_h;
                false
            }
            _ => {
                self.disclosed = Some(Disclosed {
                    key,
                    open,
                    body_h,
                    closing: None,
                    at: None,
                    moving: false,
                });
                false
            }
        };
        if let Some(d) = self.disclosed.as_mut()
            && d.moving
            && !opened
            && motion::progress(d.at, self.now_ms, motion::DISCLOSE_MS) >= 1.0
        {
            d.moving = false;
        }
        opened
    }

    /// Brings whatever is moving to rest at once: a glide lands, a
    /// stretch ends, a toast is fully up, the overlay scrollbar is gone
    /// and a text caret shows. For drivers that take pictures of the screens,
    /// such as the snapshot tool, so a picture shows where things end up.
    pub fn settle(&mut self) {
        self.caret_at = self.now_ms;
        let _ = self.with_region(|m, r| m.settle(r));
        self.motion.stop();
        self.scroll_at = None;
        if let Some(d) = self.disclosed.as_mut() {
            d.moving = false;
        }
        self.guided_moving = None;
        let f = self.f.max(0.1);
        let offset = self.scroll_slot().map_or(0.0, |o| (*o * f).round() / f);
        self.bar_seen = ((self.region_key(), self.slot()), offset, 0);
        if self.toast.is_some() {
            self.toast_at = Some(self.now_ms.saturating_sub(motion::TOAST_MS));
        }
        self.dirty = true;
    }

    /// Acts as if `action` had been pressed: for drivers that script the
    /// screens, such as the snapshot tool.
    pub fn press(&mut self, action: Action) {
        self.note_secrets();
        self.act(action);
        self.backup_settle();
        // A vault being made or unlocked for another flow: the way back
        // ends when the person leaves for anything else, and a secret
        // kept for it goes, wiped. While on the way, the flow keeps what
        // it held.
        let detour = matches!(
            self.screen,
            Screen::CreateVault | Screen::Vaults | Screen::Unlock
        );
        if !detour {
            self.vaults.back_to = None;
        }
        let waiting = detour && self.vaults.back_to.is_some();
        if !waiting && !matches!(self.sheet, Some(Sheet::SecretOut | Sheet::Pull)) {
            self.secret_cancel();
        }
        let kept = |s: Screen| self.screen == s || (waiting && self.vaults.back_to == Some(s));
        let (bip85, lightning, silent) = (
            kept(Screen::Bip85),
            kept(Screen::Lightning),
            kept(Screen::Silent),
        );
        // Leaving the new-key screen drops what it held.
        if self.screen != Screen::KeyGen && self.keygen.is_some() {
            self.keygen = None;
        }
        if !bip85 {
            self.bip85 = None;
        }
        if self.screen != Screen::Explore {
            self.explore = None;
        }
        if self.screen != Screen::Tools {
            self.tools = None;
        }
        // Leaving the upgrade ends it: the copier drops the source and
        // the boot partitions go back to root.
        if self.screen != Screen::Upgrade {
            self.upgrade_leave();
        }
        if self.sheet != Some(Sheet::WordList) {
            self.wordlist = None;
        }
        // Leaving the vanity screen stops a search and drops a find.
        if self.screen != Screen::Vanity {
            self.vanity = None;
        }
        if !lightning && self.sheet != Some(Sheet::SecretOut) {
            self.lightning = None;
        }
        if !silent && self.sheet != Some(Sheet::Qr) {
            self.silent = None;
        }
        self.keygen_camera();
        self.keygen_device();
        let spent = self.spend.as_mut().filter(|s| s.round_spent).map(|s| {
            s.round_spent = false;
            s.spend.psbt.clone()
        });
        if let Some(psbt) = spent {
            self.vault_round_used(&psbt);
        }
        self.release_held_stick();
        // A device still waiting comes back up once no other sheet is.
        self.input_sheet();
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    /// Whether the last frame drawn offers `action` to a press: for
    /// tests that a control is on screen.
    pub fn offers(&self, action: Action) -> bool {
        self.hits.iter().any(|(_, a)| *a == action)
    }

    /// Where the last frame drawn offers `action`, in pixels: the middle
    /// of where it can be pressed, for tests that touch it.
    pub fn where_offered(&self, action: Action) -> Option<(u16, u16)> {
        let (r, _) = self.hits.iter().rev().find(|(_, a)| *a == action)?;
        Some(((r.x + r.w / 2) as u16, (r.y + r.h / 2) as u16))
    }

    /// The box the last frame drawn offers `action` in, in pixels
    /// (x, y, w, h): for a test that two controls do not overlap.
    pub fn hit_box(&self, action: Action) -> Option<(u16, u16, u16, u16)> {
        let (r, _) = self.hits.iter().rev().find(|(_, a)| *a == action)?;
        Some((r.x as u16, r.y as u16, r.w as u16, r.h as u16))
    }

    /// The width and height of the layout, in design units.
    pub fn size(&self) -> (f32, f32) {
        (self.w, self.h)
    }

    /// The keys loaded that no wallet uses, and the one of them the
    /// Wallets card offers a wallet from: the one picked, else the first.
    pub fn loose_keys(&self) -> (Vec<osk_bip::keys::Fingerprint>, Option<[u8; 4]>) {
        let loose = self.session.loose_keys();
        let picked = self
            .loose_pick
            .filter(|p| loose.iter().any(|f| f.0 == *p))
            .or_else(|| loose.first().map(|f| f.0));
        (loose, picked)
    }

    /// Whether the display is a small panel, drawn one column at a time.
    pub fn is_compact(&self) -> bool {
        self.compact
    }
}

/// The create cards a kind shows.
pub fn create_steps(kind: create::NewKind) -> Vec<u8> {
    let mut v = vec![cstep::KIND];
    if kind.multi() {
        v.push(cstep::QUORUM);
    }
    v.extend([cstep::KEYS, cstep::CHECK, cstep::BACKUP]);
    v
}

/// The stem a wallet's public files are named by: its name in lower case,
/// with anything but letters and digits as a hyphen.
pub fn file_stem(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The name a result file takes from the PSBT it came from: the stem,
/// without an `-unsigned` the coordinator put on it.
pub fn result_stem(source: &str) -> String {
    let s = stem(source);
    ["-unsigned", "-partly-signed"]
        .iter()
        .find_map(|end| s.strip_suffix(end))
        .map(str::to_string)
        .unwrap_or(s)
}

/// Whether pressing `a` is pressing a text field, which a second press
/// or a drag selects all of.
fn field_action(a: Action) -> bool {
    use vaults::VaultAction as V;
    matches!(
        a,
        Action::Vault(
            V::FocusPassphrase
                | V::CFocus(..)
                | V::CName
                | V::FocusField(_)
                | V::FocusPrompt
                | V::Dice(_)
        ) | Action::EntryPassphrase
            | Action::KPassField(_)
            | Action::CPassField(_)
            | Action::RPassField(_)
            | Action::Rename
            | Action::MType
            | Action::KTyping(true)
            | Action::Seeds(seeds::SeedsAction::Focus(_))
    )
}

/// A file name without its extension.
pub(crate) fn stem(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 => name[..i].to_string(),
        _ => name.to_string(),
    }
}

impl App for Faraday {
    fn event(&mut self, event: Event) {
        match event {
            Event::Display(d) => self.display(d),
            Event::Touch { x, y, phase } => {
                self.input_now();
                self.caret_at = self.now_ms;
                // Any input puts the idle lock off; the press that does
                // it does nothing else.
                if self.sheet == Some(Sheet::IdleWarn) {
                    if phase == TouchPhase::Down {
                        self.sheet = None;
                        self.pressed = None;
                    }
                } else {
                    self.touch(x, y, phase);
                }
            }
            Event::Key(k) => {
                self.input_now();
                self.caret_at = self.now_ms;
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.sheet = None;
                } else {
                    self.key(k);
                }
            }
            Event::Shift { held } => {
                self.shift = held;
                return;
            }
            Event::Back => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.sheet = None;
                } else {
                    self.mouse_back();
                }
            }
            Event::Forward => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.sheet = None;
                } else {
                    self.mouse_forward();
                }
            }
            Event::Scroll { x, y, dy } => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.sheet = None;
                }
                // A gesture goes on moving the region it started over.
                if self.scroll_at.is_none() {
                    self.point_region(i32::from(x), i32::from(y));
                }
                // Pixels the content moves up, as the shell API states
                // them, in this layout's units.
                self.pan(f32::from(dy) / self.f.max(0.1));
                self.scroll_at = Some(self.now_ms);
            }
            Event::Wheel { x, y, dy } => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.sheet = None;
                }
                self.point_region(i32::from(x), i32::from(y));
                self.glide(f32::from(dy) / self.f.max(0.1));
            }
            Event::ScrollEnd { .. } => {
                self.scroll_at = None;
                self.motion.release(self.f);
            }
            // A frame only when what is under the pointer changes.
            Event::Hover { x, y } => {
                self.input_now();
                let was = self.hover.is_some_and(|(x, y)| self.bar_at(x, y).is_some());
                self.hover = Some((i32::from(x), i32::from(y)));
                let now = self.hover.is_some_and(|(x, y)| self.bar_at(x, y).is_some());
                // Coming over the scrollbar shows it.
                if !self.find_hovered() && was == now {
                    return;
                }
            }
            Event::HoverEnd => {
                self.hover = None;
                if !self.find_hovered() {
                    return;
                }
            }
            Event::Tick { now_ms } => {
                // The scrollbar was showing at the last tick: this one
                // draws its next step of fading, the last one included.
                let fading = motion::bar_alpha(self.bar_seen.2, self.now_ms) > 0;
                self.now_ms = now_ms;
                // Asked for on every tick while something moves, so the
                // shell keeps drawing at its frame rate.
                // A card opening or a toast coming or going starts on its
                // first tick and wants a frame on each.
                if let Some(d) = self.disclosed.as_mut()
                    && d.moving
                    && d.at.is_none()
                {
                    d.at = Some(now_ms);
                }
                if let Some((_, at @ None)) = self.guided_moving.as_mut() {
                    *at = Some(now_ms);
                }
                if let Some((_, Some(at))) = self.guided_moving
                    && now_ms >= at + motion::SWITCH_MS + 16
                {
                    self.guided_moving = None;
                }
                if self.toast.is_some() && self.toast_at.is_none() {
                    self.toast_at = Some(now_ms);
                }
                let fading = fading
                    || self.caret_drawn.is_some_and(|on| on != self.caret_on())
                    || self.disclosed.is_some_and(|d| d.moving)
                    || self.guided_moving.is_some()
                    || self.toast.as_ref().is_some_and(|(_, until)| {
                        self.toast_at
                            .is_some_and(|at| now_ms < at + motion::TOAST_MS + 16)
                            || now_ms + motion::TOAST_MS + 16 > *until
                    });
                // Scrolling that stopped with no end said (a shell that
                // never sends one) ends here, so a stretch springs back.
                if self
                    .scroll_at
                    .is_some_and(|at| now_ms.saturating_sub(at) > motion::SCROLL_QUIET_MS)
                {
                    self.scroll_at = None;
                    self.motion.release(self.f);
                }
                if self.motion_tick(now_ms) || fading {
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                }
                if self.idle_tick(now_ms) {
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                    return;
                }
                if self.vault_tick(now_ms) {
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                }
                self.vanity_tick(now_ms);
                if let Some(q) = self.qr.as_mut()
                    && q.frames.len() > 1
                    && now_ms >= q.next_ms
                {
                    q.frame = (q.frame + 1) % q.frames.len();
                    q.next_ms = now_ms + self.qr_frame_ms;
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                }
                if let Some((_, until)) = &self.toast
                    && now_ms > *until
                {
                    self.toast = None;
                } else {
                    return;
                }
            }
            Event::Entropy(e) => {
                let mut bytes = e.into_bytes();
                // Asked for by New key: the bytes are the key's, and the
                // session's own seed stays as it was.
                if !self.keygen_entropy(&mut bytes) {
                    self.fresh_arrived(bytes);
                    zeroize::Zeroize::zeroize(&mut bytes);
                }
                return;
            }
            Event::CameraFrame { luma, .. } if self.keygen_camera_on => {
                if let Some(k) = self.keygen.as_mut() {
                    if let Some(old) = k.frame.as_mut() {
                        zeroize::Zeroize::zeroize(old);
                    }
                    k.frame = Some(luma);
                }
            }
            Event::CameraFrame {
                width,
                height,
                luma,
                chroma,
            } => match self.scan.as_mut() {
                Some(s) => {
                    if let Some((_, _, old)) = s.frame.as_mut() {
                        zeroize::Zeroize::zeroize(old);
                    }
                    if let Some(old) = s.chroma.as_mut() {
                        zeroize::Zeroize::zeroize(old);
                    }
                    s.frame = Some((width, height, luma));
                    s.chroma = chroma;
                    s.frames = s.frames.wrapping_add(1);
                }
                None => return,
            },
            Event::Scanned { bytes } => {
                self.scanned(bytes);
                self.release_held_stick();
            }
            Event::CameraUnavailable => {
                if self.scan.take().is_some() {
                    self.sheet = None;
                    self.toast("No camera, or the camera was refused");
                }
                self.release_held_stick();
            }
            _ => return,
        }
        self.backup_settle();
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.note_secrets();
        self.commands.pop_front()
    }

    fn frame(&mut self) -> Frame<'_> {
        if self.dirty {
            self.dirty = false;
            self.render();
        }
        match self.canvas.as_ref() {
            Some(c) => c.frame(),
            None => Frame {
                width: 0,
                height: 0,
                rgba: &[],
            },
        }
    }
}

impl Faraday {
    /// The strings the current frame draws, for a test to check what a
    /// title or a label actually reads.
    pub fn drawn_texts(&mut self) -> Vec<String> {
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(true);
        }
        self.render();
        let texts = self
            .canvas
            .as_ref()
            .map(|c| {
                c.ink()
                    .iter()
                    .filter_map(|i| match &i.kind {
                        osk_ui::canvas::InkKind::Text { text, .. } => Some(text.clone()),
                        osk_ui::canvas::InkKind::Icon(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(false);
        }
        self.dirty = true;
        texts
    }

    fn display(&mut self, d: DisplayInfo) {
        self.last_display = Some(d);
        // The self-test runs once, at start, before any input: a display
        // comes before anything can be pressed.
        if self.selftest.is_none() {
            self.selftest = Some(osk_selftest::run());
        }
        self.frost = None;
        // Fill a small display; on a large one, stop at a quarter more
        // than its density asks for, so a 4K monitor shows the screens
        // at a sensible size rather than magnified. The person's own
        // choice multiplies whatever that gives.
        let fit = (f32::from(d.width) / 1280.0).min(f32::from(d.height) / 800.0);
        let density = f32::from(d.dpi.max(1)) / 160.0;
        // A panel under 600 dp wide (the Pi's 2.8" is 268) gets the
        // one-column layout at its own density, not the desktop shrunk.
        self.compact = f32::from(d.width) / density < 600.0;
        let base = if self.compact {
            density
        } else {
            fit.min(density * 1.25)
        };
        let f = (base * f32::from(self.scale_pct) / 100.0).max(0.5);
        let scaled = DisplayInfo {
            dpi: (160.0 * f).round().max(1.0) as u16,
            ..d
        };
        self.f = f;
        self.w = f32::from(d.width) / f;
        self.h = f32::from(d.height) / f;
        self.canvas = Some(Canvas::new(&scaled));
        self.fresh_ask();
    }
}

/// Whether a pass over this transaction may open a MuSig2 or FROST
/// round, which draws secret nonces: any such input is one, since what
/// the pass does with it is decided only as it signs.
fn spend_opens_round(inspection: &osk_psbt::Inspection) -> bool {
    inspection
        .inputs
        .iter()
        .any(|i| i.musig.is_some() || i.threshold.is_some())
}
