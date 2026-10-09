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
pub mod bip85;
pub mod boot_import;
pub mod catalog;
pub mod create;
pub mod decode;
pub mod explore;
pub mod family;
pub mod flow;
pub mod forms;
mod fresh;
pub mod gpg;
pub mod inbox;
pub mod inputs;
pub mod keygen;
pub mod learn;
pub mod lightning;
pub mod memory;
pub mod paper;
pub mod pdf;
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
pub mod ui;
pub mod vanity;
pub mod vaults;
pub mod wallet;
pub mod wordlist;

mod bip85_screen;
mod boot_import_screen;
mod compact;
pub use compact::OskPress;
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
mod vanity_screen;
mod vault_screens;
mod wordlist_screen;

use std::collections::{BTreeSet, VecDeque};

use osk_shell_api::{App, Command, DisplayInfo, Event, Frame, Key as KeyIn, TouchPhase};
use osk_ui::{Canvas, Rect};

use wallet::{FileKind, Session, Spend, fp_text};

/// The version shown on Settings.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
    /// The QR codes read from an image on a stick.
    QrRead {
        /// The image's name.
        name: String,
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
}

/// A screen and the sheet over it, if any.
type ScreenKey = (Screen, Option<Sheet>);

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
    /// Put the PSBT with the signatures so far in the Outbox.
    PartToOutbox,
    /// Put the signed PSBT in the Outbox.
    SignedToOutbox,
    /// Put the finished transaction in the Outbox.
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
    /// Choose every file on the stick Faraday reads, or none when all are
    /// chosen.
    VisitInAll,
    /// The stick's file list's scrollbar: held and dragged.
    VisitBar,
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
    /// Open the network chooser.
    NetworkAsk,
    /// Minutes without input before locking; 0 for never.
    IdleLock(u16),
    /// Minutes without input before powering off; 0 for never.
    IdleOff(u16),
    /// Believe a new pointer, said with input already believed.
    InputUse(u32),
    /// Keep a new device out until it is unplugged.
    InputIgnore(u32),
    /// Save the wallet just made, and its keys held here, into the open
    /// vault.
    CSaveAll,
    /// One of the wallet just made's public files to the Outbox, as
    /// `public_out` numbers them.
    CPublic(u8),
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
    /// The QR view as BBQr (`true`) or UR.
    QrFormat(u8),
    /// The QR view's bytes of data a frame.
    QrPartSize(usize),
    /// Milliseconds each frame of an animated code stays up.
    QrSpeed(u64),
    /// Open a wallet's card.
    OpenWallet(usize),
    /// Start backing up a wallet.
    Backup(usize),
    /// Open or close a backup card.
    BStep(u8),
    /// Close a backup card as done and open the next.
    BNext(u8),
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
    /// Clear the typed digits.
    BCheckClear,
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
    /// Load the key.
    KAdd,
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
    /// Make the wallet.
    CMake,
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
    /// Restore from the seeds alone: the seeds card opens in seeds-first
    /// mode.
    RSeeds,
    /// A wallet from a loaded key in no wallet, by fingerprint: Restore's
    /// seeds card with that key in, at the shape, with this many keys
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
    /// Put a backup file in the Outbox: 0 template, 1 descriptor,
    /// 2 multisig config, 3 backup sheet, 4 split shares, 5 wallet .json.
    BOut(u8),
    /// Put the backup's sheets in the Outbox: the template and the sheet.
    BSheets,
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
        }
    }

    /// Who may read it: a secret let out is one, whatever its kind.
    pub fn exposure(&self) -> secrets::Exposure {
        if self.secret {
            secrets::Exposure::Secret
        } else {
            self.kind.exposure()
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
    /// Choosing the network.
    Network,
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
    /// Scanning a SeedQR for Add a key, not a file for Files.
    pub key: bool,
    /// Scanning one part of a key for Add a key's other forms.
    pub part: bool,
    /// Scanning a TOTP setup code into a vault entry.
    pub entry: bool,
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
        })
    }

    /// The same source in another format or part size.
    pub fn redo(&self, format: QrFormat, part: usize) -> Result<QrView, String> {
        let secret = self.secret;
        QrView::of(&self.title, self.source.clone(), format, part).map(|mut v| {
            v.secret |= secret;
            v
        })
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
}

/// The backup flow's state.
#[derive(Default)]
pub struct BackupState {
    /// The wallet being backed up.
    pub wallet: usize,
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; 5],
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
    /// Template word count.
    pub words: usize,
    /// Keys left off each split sheet.
    pub omit: usize,
    /// What went to the Outbox, for the summary.
    pub sent: Vec<String>,
    /// Another paper form of the key, on screen.
    pub paper: Option<paper::PaperForm>,
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
    pub done: [bool; 8],
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
    /// New keys made for this wallet.
    pub made: u32,
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
    /// The keys.
    pub const KEYS: u8 = 2;
    /// The descriptor, and making it.
    pub const BUILD: u8 = 3;
    /// The first addresses.
    pub const CHECK: u8 = 4;
    /// The paper backup: seeds by hand, sheets and envelopes.
    pub const BACKUP: u8 = 5;
    /// The secrets: the keys held here, and the wallet, into a vault.
    pub const VAULT: u8 = 6;
    /// The public files: descriptor, wallet file, config, sheet, keys.
    pub const PUBLIC: u8 = 7;
}

/// Restoring a wallet: the cards' state.
#[derive(Default)]
pub struct RestoreState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; 5],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The Inbox shares chosen, by name.
    pub shares: Vec<String>,
    /// The wallet, once restored, by index into the session.
    pub wallet: Option<usize>,
    /// The last refusal.
    pub error: Option<String>,
    /// Restoring from the seeds alone, before any wallet: the seeds
    /// typed so far and the wallet they will make.
    pub seeds: Option<seeds::SeedsState>,
    /// Opened from a key on Wallets: Back goes to Wallets.
    pub from_key: bool,
}

/// The restore cards, in order.
pub mod rstep {
    /// A transaction to sign, brought in first.
    pub const PSBT: u8 = 0;
    /// The wallet's description.
    pub const WALLET: u8 = 1;
    /// The seeds, typed back.
    pub const SEEDS: u8 = 2;
    /// The first address against the sheet.
    pub const CHECK: u8 = 3;
    /// What next.
    pub const DONE: u8 = 4;
    /// How many cards.
    pub const COUNT: u8 = 5;
}

/// The backup cards, in order.
pub mod bstep {
    /// The blank template.
    pub const BLANK: u8 = 0;
    /// The seeds, by hand.
    pub const SEEDS: u8 = 1;
    /// The wallet in public keys.
    pub const PUBLIC: u8 = 2;
    /// The split between signers.
    pub const SPLIT: u8 = 3;
    /// The envelope.
    pub const ENVELOPE: u8 = 4;
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
    /// Where the file list's scrollbar was drawn, in pixels: the track's
    /// top and height, the thumb's height, and the furthest the list
    /// scrolls in design units.
    pub bar: std::cell::Cell<Option<(i32, i32, i32, f32)>>,
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
    /// A secret has been held or typed in this process. Only a fresh
    /// process is clean (`PLAN.md` §5.1).
    tainted: bool,
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
    /// The vaults: the open slots and the vault screens' state.
    pub vaults: vaults::Vaults,
    /// The amounts each signed transaction's inputs stated
    /// (`docs/WALLETS.md` §3.3).
    pub signed_amounts: Vec<memory::Signed>,
    /// Seal the signed-amount memory into an open vault on lock.
    pub seal_amounts: bool,
    seed: [u8; 32],
    /// The session's seed has arrived.
    seeded: bool,
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
    /// The height a scrolled page last drew to, design units: how far its
    /// scroll may go.
    pub(crate) content_h: std::cell::Cell<f32>,
    /// What moves the scrolled region: pans, glides, coasts and the
    /// stretch at its ends (`docs/MOTION.md` §3.3).
    motion: motion::Motion,
    /// The screen and sheet the motion belongs to.
    motion_for: ScreenKey,
    /// The scrolled region the last frame drew, and the screen and sheet
    /// it was drawn for.
    extent: Option<(ScreenKey, ui::Scrolled)>,
    /// For the overlay scrollbar: the screen and sheet, the offset last
    /// drawn, and when it last changed (0 for not since it opened).
    bar_seen: (ScreenKey, f32, u64),
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
    /// The cameras the shell can open.
    pub cameras: Vec<(String, String)>,
    /// The one chosen, by id; the shell's first when none is.
    pub camera: Option<String>,
    /// A camera chosen that the shell has not opened yet.
    camera_change: Option<String>,
    /// The camera sheet.
    pub scan: Option<ScanState>,
    /// The chosen wallet's new name, while it is typed.
    pub renaming: Option<String>,
    /// All of the focused field is selected: Backspace clears it, and a
    /// typed character replaces it.
    pub select_all: bool,
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
            tainted: false,
            inputs: Vec::new(),
            ignored_inputs: Vec::new(),
            import_under_input: false,
            input_decisions: VecDeque::new(),
            new_keys: 0,
            vaults: vaults::Vaults::default(),
            signed_amounts: Vec::new(),
            seal_amounts: true,
            seed: [0; 32],
            seeded: false,
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
            guided: false,
            about_open: None,
            pin_h: std::cell::Cell::new(0.0),
            list_offset: 0.0,
            content_h: std::cell::Cell::new(0.0),
            motion: motion::Motion::default(),
            motion_for: (Screen::Home, None),
            extent: None,
            bar_seen: ((Screen::Home, None), 0.0, 0),
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
            scanned: 0,
            renaming: None,
            select_all: false,
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
        // A device still waiting comes back up once no other sheet is.
        self.input_sheet();
    }

    fn storage_event(&mut self, event: StorageEvent) {
        let event = match self
            .settings_event(event)
            .and_then(|e| self.import_event(e))
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
            StorageEvent::Read { name, bytes, .. } => {
                // A file Faraday reads as nothing else still comes in, as
                // a File: to sign, or to send as codes.
                let item = Item::new(&name, bytes);
                let line = if item.kind == FileKind::Words && self.visit.load_after.contains(&name)
                {
                    format!("Copied {name}: a key's words, loaded when the stick is removed")
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
                self.outbox.retain(|i| i.name != name);
                self.visit.out.remove(&name);
                let line = if wrote_as == name {
                    format!("Wrote {name}, read back and matched")
                } else {
                    format!("Wrote {name} as {wrote_as}, read back and matched")
                };
                self.visit.log.push((line, true));
                self.save_boxes();
            }
            StorageEvent::WriteFailed { name, reason, .. } => {
                self.visit.log.push((
                    format!("{name} not written: {reason}. It stays in the Outbox"),
                    false,
                ));
            }
            StorageEvent::Printed { path } => self.toast(&format!("Saved {path}")),
            StorageEvent::Clock { unix_secs } => {
                self.vaults.unix_secs = Some(unix_secs);
                return;
            }
            StorageEvent::Memory { available_mib } => {
                self.vaults.memory_free_mib = Some(available_mib);
                if !matches!(self.screen, Screen::CreateVault | Screen::Unlock) {
                    return;
                }
            }
            StorageEvent::Cameras(list) => {
                self.cameras = list;
                return;
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
            StorageEvent::QrRead { name, payloads } => {
                self.visit.inn.remove(&name);
                if payloads.is_empty() {
                    self.visit
                        .log
                        .push((format!("{name}: no QR code found"), false));
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
                            let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
                            let got = self.free_inbox_name(&format!("{stem}-words.txt"));
                            self.inbox.push(Item::new(&got, words.as_bytes().to_vec()));
                            self.visit.load_after.insert(got.clone());
                            self.save_boxes();
                            seeds += 1;
                            self.visit.log.push((
                                format!(
                                    "{name}: a SeedQR, copied in as {got}; its key loads when the \
                                     stick is removed"
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
                        let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
                        let taken: Vec<String> =
                            self.inbox.iter().map(|i| i.name.clone()).collect();
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
            StorageEvent::PrintFailed { reason } => self.toast(&format!("Not printed: {reason}")),
            StorageEvent::NewInput {
                id,
                name,
                keyboard,
                pointer,
            } => self.input_new(id, name, keyboard, pointer),
            StorageEvent::InputTyped { id, ch } => self.input_typed(id, ch),
            StorageEvent::InputGone { id } => self.input_gone(id),
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
    pub fn clean(&mut self) -> bool {
        self.note_secrets();
        !self.tainted
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
            self.not_now = false;
            if self.sheet == Some(Sheet::Lock) {
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
                self.screen = self.after_visit.take().unwrap_or(Screen::Home);
                if self.screen == Screen::Family {
                    self.family_settle();
                }
            }
            if had {
                let loaded = self.load_after_pull();
                let said = if loaded == 0 {
                    "Stick removed".to_string()
                } else {
                    format!(
                        "Stick removed · {loaded} {} loaded",
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
            return;
        }
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
            self.sheet = Some(Sheet::Lock);
        } else {
            self.visit.out = self.visit_default_out();
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
    /// be pulled first, when it loads a key; `None` for anything a stick
    /// may stay in for.
    pub fn pull_what(&self, action: Action) -> Option<&'static str> {
        use catalog::Go;
        Some(match action {
            Action::Entry(_) | Action::ScanSeed | Action::ScanPart | Action::LoadKey(_) => {
                "add a key"
            }
            Action::KeyGen(_) | Action::KeyGenSlip39 => "make a key",
            Action::PotentialOpen(_) => "load the wallet",
            Action::BackupOpen(_) => "open the backup",
            Action::Catalog(i) => match catalog::TILES.get(usize::from(i))?.go {
                Go::NewKey | Go::NewShares => "make a key",
                Go::AddKey(_) | Go::SeedQr => "add a key",
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

    /// What a visit writes unless the person changes it: the Outbox but
    /// an unprotected secret, which is written only when ticked on the
    /// visit itself.
    fn visit_default_out(&self) -> std::collections::BTreeSet<String> {
        self.outbox
            .iter()
            .filter(|i| i.exposure() != secrets::Exposure::Secret)
            .map(|i| i.name.clone())
            .collect()
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
        self.inbox.retain(|i| {
            !matches!(
                i.kind,
                FileKind::Words
                    | FileKind::SeedPart
                    | FileKind::Carry
                    | FileKind::Entries
                    | FileKind::Text
                    | FileKind::Other
            ) && !i.secret
        });
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
            open: Some(1),
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
            s.steps = kind.steps(wallet::txid_known(&s.spend.psbt));
            s.complete = s.spend.finished.is_some() || s.spend.finish().is_ok();
            if let Some(open) = s.open
                && !s.steps.contains(&open)
            {
                s.open = s.steps.iter().copied().find(|&n| !s.done[n as usize]);
            }
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
            Action::Vault(v) => self.vault_act(v),
            Action::Import(a) => self.import_act(a),
            Action::Family(f) => self.family_act(f),
            Action::Vanity(v) => self.vanity_act(v),
            Action::Learn => self.learn_open(),
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
            Action::Nav(s) => {
                self.screen = s;
                self.osk_leave();
                self.list_offset = 0.0;
                self.renaming = None;
                if s == Screen::Visit {
                    self.visit.out = self.visit_default_out();
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
                }
            }
            Action::EntryPart => self.form_add_part(),
            Action::EntryRecover => self.form_recover(),
            Action::PickWallet(i) => {
                self.wallet = i;
                self.renaming = None;
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
            Action::PartToOutbox => {
                if let Some(s) = self.spend.as_ref() {
                    let name = format!("{}-part.psbt", result_stem(&s.spend.source));
                    let bytes = s.spend.psbt.to_bytes();
                    self.put_outbox(&name, bytes);
                    self.toast(&format!("{name} is in the Outbox"));
                }
            }
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
                self.visit.inn.clear();
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
                    && let Some((n, _)) = stick.files.get(i)
                {
                    let n = n.clone();
                    if !self.visit.inn.remove(&n) {
                        self.visit.inn.insert(n);
                    }
                }
            }
            Action::VisitInAll => {
                if let Some(stick) = self.sticks.get(self.visit.stick) {
                    let all: BTreeSet<String> = stick
                        .files
                        .iter()
                        .filter(|(n, _)| stick_kind(n).is_some())
                        .map(|(n, _)| n.clone())
                        .collect();
                    if !all.is_empty() && all.is_subset(&self.visit.inn) {
                        self.visit.inn.clear();
                    } else {
                        self.visit.inn = all;
                    }
                }
            }
            Action::VisitBar => {}
            Action::VisitWrite => {
                if let Some(stick) = self.sticks.get(self.visit.stick) {
                    let id = stick.id.clone();
                    self.visit.log.clear();
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
                self.not_now = true;
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
            Action::NetworkAsk => self.sheet = Some(Sheet::Network),
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
            Action::CSaveAll => self.create_save_all(),
            Action::CPublic(what) => {
                if let Some(i) = self.create.as_ref().and_then(|c| c.built) {
                    self.public_out(i, what);
                }
            }
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
                }
            }
            Action::Backup(i) => {
                if i < self.session.wallets.len() {
                    let first_key = self.backup_keys(i).first().copied().unwrap_or(0);
                    let (m, _) = Session::quorum(&self.session.wallets[i]);
                    self.backup = Some(BackupState {
                        wallet: i,
                        open: Some(bstep::BLANK),
                        key: first_key,
                        words: 24,
                        omit: m.saturating_sub(1),
                        ..BackupState::default()
                    });
                    self.screen = Screen::Backup;
                }
            }
            Action::BStep(n) => {
                if let Some(b) = self.backup.as_mut() {
                    b.open = if b.open == Some(n) { None } else { Some(n) };
                    b.scroll.follow = true;
                    b.checking = false;
                }
            }
            Action::BNext(n) => {
                let steps = self.backup_steps();
                if let Some(b) = self.backup.as_mut() {
                    b.done[n as usize] = true;
                    b.open = steps.iter().copied().find(|&i| !b.done[i as usize]);
                    b.scroll.follow = true;
                    b.checking = false;
                }
            }
            Action::BKey(k) => {
                if let Some(b) = self.backup.as_mut() {
                    b.key = k;
                    b.reveal = false;
                    b.typed.clear();
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
            Action::BCheckClear => {
                if let Some(b) = self.backup.as_mut() {
                    b.typed.clear();
                }
            }
            Action::BWords(n) => {
                if let Some(b) = self.backup.as_mut() {
                    b.words = n;
                }
            }
            Action::BOmit(n) => {
                if let Some(b) = self.backup.as_mut() {
                    b.omit = n;
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
                    self.toast("Remove the stick first");
                }
            }
            Action::XpubOpen(i) => self.xpub_open(i),
            Action::BackupOpen(k) => {
                if self.may_load_keys() {
                    self.backup_open(k);
                } else {
                    self.toast("Remove the stick first");
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
            Action::BSheets => {
                self.backup_out(0);
                self.backup_out(3);
            }
            Action::WriteAsk => {
                if !self.clean() {
                    self.sheet = Some(Sheet::WriteOut);
                } else if self.sticks.is_empty() {
                    self.toast("Plug in a stick: the visit writes the Outbox");
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
                        self.toast(&format!("{name} is in the Outbox"));
                    } else {
                        self.open_qr(QrView::text("Signed message", &text));
                    }
                }
            }
            Action::CreateWallet if self.create_unfinished() => {
                self.screen = Screen::Create;
            }
            Action::CreateWallet | Action::CreateOver => {
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
            Action::CNext(k) => {
                if let Some(c) = self.create.as_mut() {
                    c.done[k as usize] = true;
                    let steps = create_steps(c.kind);
                    c.open = steps.iter().copied().find(|&i| !c.done[i as usize]);
                    c.scroll.follow = true;
                }
            }
            Action::CKind(i) => {
                if let Some(c) = self.create.as_mut()
                    && c.built.is_none()
                {
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
                            let account = self.create_key_account(k);
                            self.open_qr(QrView::of(
                                &format!("Xpub {fp} · {kind}"),
                                QrSource::Key(text, account),
                                QrFormat::Ur,
                                QR_PARTS[1],
                            ));
                        } else {
                            let name = format!("xpub-{fp}.txt");
                            self.put_outbox(
                                &name,
                                format!("# Account xpub {fp}, {kind}\n{text}\n").into_bytes(),
                            );
                            self.toast(&format!("{name} is in the Outbox"));
                        }
                    }
                    Err(e) => self.toast(&e),
                }
            }
            Action::CKeyBsms(k) => {
                let made = self.create.as_ref().and_then(|c| {
                    let Some(create::Source::Here(fp)) = c.slots.get(k as usize) else {
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
                        .map(|r| (format!("xpub-{fpt}-bsms.txt"), r))
                });
                match made {
                    Some((name, text)) => {
                        self.put_outbox(&name, text.into_bytes());
                        self.toast(&format!("{name} is in the Outbox"));
                    }
                    None => self.toast("BIP 129 covers wsh and sh(wsh) multisig keys"),
                }
            }
            Action::CSlotClear(k) => {
                if let Some(c) = self.create.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Empty;
                }
            }
            Action::CMake => self.create_make(),
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
                    scan.part = true;
                    self.scan = Some(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.commands.push_back(Command::CameraOn);
                }
            }
            Action::ScanSeed => {
                if self.may_load_keys() {
                    let mut scan = ScanState::default();
                    scan.key = true;
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
                self.restore = Some(RestoreState {
                    open: Some(rstep::PSBT),
                    ..RestoreState::default()
                });
                self.screen = Screen::Restore;
            }
            Action::RStep(k) => {
                if let Some(r) = self.restore.as_mut() {
                    r.open = if r.open == Some(k) { None } else { Some(k) };
                    r.scroll.follow = true;
                }
            }
            Action::RNext(k) => {
                if let Some(r) = self.restore.as_mut() {
                    r.done[k as usize] = true;
                    r.open = (0..rstep::COUNT).find(|&i| !r.done[i as usize]);
                    r.scroll.follow = true;
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
                if let Some(r) = self.restore.as_mut() {
                    r.seeds.get_or_insert_with(seeds::SeedsState::default);
                    // The wallet comes from the seeds now, not a file.
                    r.wallet = None;
                    r.error = None;
                    r.done[rstep::WALLET as usize] = false;
                    r.done[rstep::SEEDS as usize] = false;
                    r.done[rstep::CHECK as usize] = false;
                    r.open = Some(rstep::SEEDS);
                    r.scroll.follow = true;
                }
            }
            Action::KeyWallet(fp, n) => {
                let mut s = seeds::SeedsState::default();
                s.take(fp);
                s.start_shape();
                s.set_n(usize::from(n));
                self.restore = Some(RestoreState {
                    open: Some(rstep::SEEDS),
                    seeds: Some(s),
                    from_key: true,
                    ..RestoreState::default()
                });
                self.screen = Screen::Restore;
            }
            Action::Seeds(a) => self.seeds_act(a),
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
                if let Some(w) = self.session.wallets.get(i) {
                    let (title, text) = match w.policy.record() {
                        Some(r) => (format!("{} · threshold record", w.name), r.to_text()),
                        None => (
                            format!("{} · wallet descriptor", w.name),
                            w.policy.to_descriptor_checksummed(),
                        ),
                    };
                    self.open_qr(QrView::text(&title, &text));
                }
            }
            Action::QrOutbox(i) => {
                if let Some(item) = self.outbox.get(i).filter(|it| qr_fits(it)) {
                    // What a wallet reads goes as itself; any other file in
                    // the Faraday file envelope.
                    let source = match item.kind {
                        FileKind::Psbt => QrSource::Psbt(item.bytes.clone()),
                        FileKind::Wallet | FileKind::Key | FileKind::Message | FileKind::Share => {
                            QrSource::Text(String::from_utf8_lossy(&item.bytes).into_owned())
                        }
                        _ => QrSource::File(item.name.clone(), item.bytes.clone()),
                    };
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
                    Some(secrets::SecretOut {
                        name: format!("{stem}-partly-signed.osk"),
                        bytes,
                        what: "The secret nonce the next share signs this transaction with",
                        gives: "Whoever has it and a share's signature can work out that share",
                        round: Some(round),
                    })
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

    /// The backup cards this wallet shows.
    pub fn backup_steps(&self) -> Vec<u8> {
        let multi = self
            .backup
            .as_ref()
            .and_then(|b| self.session.wallets.get(b.wallet))
            .is_some_and(backup::splits);
        let mut v = vec![bstep::BLANK, bstep::SEEDS, bstep::PUBLIC];
        if multi {
            v.push(bstep::SPLIT);
        }
        v.push(bstep::ENVELOPE);
        v
    }

    fn backup_out(&mut self, what: u8) {
        let Some(wallet) = self.backup.as_ref().map(|b| b.wallet) else {
            return;
        };
        self.public_out(wallet, what);
    }

    /// One of a wallet's public files to the Outbox: 0 the blank
    /// template, 1 the descriptor, 2 the multisig config, 3 the backup
    /// sheet, 4 the split shares, 5 the wallet .json, 6 the BIP 129
    /// descriptor record, 7 Bitcoin Core's `importdescriptors` file.
    pub(crate) fn public_out(&mut self, wallet: usize, what: u8) {
        let Some(w) = self.session.wallets.get(wallet) else {
            return;
        };
        let words = self.backup.as_ref().map_or(24, |b| b.words);
        let omit = self
            .backup
            .as_ref()
            .map_or(Session::quorum(w).0.saturating_sub(1), |b| b.omit);
        let stem = file_stem(&w.name);
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        match what {
            // The sheets go out as PDFs, ready to print anywhere: they
            // hold nothing secret.
            0 => match pdf::sheet(&backup::sheet_blank(words, Some(w), self.session.network())) {
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
                let net = self.session.network();
                if let Ok(first) = w.policy.address_at(net, false, 0) {
                    let record = osk_bip::bsms::DescriptorRecord {
                        policy: w.policy.clone(),
                        paths: vec!["/0/*".to_string(), "/1/*".to_string()],
                        first_address: first.to_string(),
                        network: net,
                    };
                    files.push((format!("{stem}-bsms.txt"), record.to_text().into_bytes()));
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
                if let Some(c) = backup::multisig_config(w, None) {
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
                for (i, row) in plan.iter().enumerate() {
                    if let Some(c) = backup::split_share(w, row) {
                        // The sheet to print beside each file.
                        let sheet = backup::sheet_share(w, i + 1, plan.len(), &c);
                        if let Ok(pdf) = pdf::sheet(&sheet) {
                            files.push((format!("{stem}-share-{}-of-{n}.pdf", i + 1), pdf));
                        }
                        files.push((format!("{stem}-share-{}-of-{n}.txt", i + 1), c.into_bytes()));
                    }
                }
            }
            _ => {}
        }
        let names: Vec<String> = files.iter().map(|(n, _)| n.clone()).collect();
        for (name, bytes) in files {
            self.put_outbox(&name, bytes);
        }
        if let Some(b) = self.backup.as_mut()
            && b.wallet == wallet
        {
            for n in &names {
                if !b.sent.contains(n) {
                    b.sent.push(n.clone());
                }
            }
        }
        match names.len() {
            0 => {}
            1 => self.toast(&format!("{} is in the Outbox", names[0])),
            k => self.toast(&format!("{k} files are in the Outbox")),
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
                        "Key {} waits for its cosigner's xpub file. Copy it in on a stick visit, \
                         then choose it for Key {} under Keys",
                        i + 1,
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
        self.wallet = i;
        if let Some(c) = self.create.as_mut() {
            c.built = Some(i);
            c.error = None;
            c.done[cstep::BUILD as usize] = true;
            c.open = Some(cstep::CHECK);
            c.scroll.follow = true;
        }
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

    fn create_save_all(&mut self) {
        let Some(i) = self.create.as_ref().and_then(|c| c.built) else {
            return;
        };
        let Some(w) = self.session.wallets.get(i) else {
            return;
        };
        let v = self.vaults.current;
        let fps: Vec<osk_bip::keys::Fingerprint> = self
            .session
            .slots(w)
            .iter()
            .filter(|s| s.held_by.is_some())
            .filter_map(|s| s.fingerprint)
            .collect();
        let shares: Vec<osk_bip::keys::Fingerprint> = self
            .session
            .shares_here(w)
            .iter()
            .map(|(_, k)| k.master.fingerprint())
            .collect();
        if !vault_screens::vault_has_wallet(self, v, w) {
            self.vault_act(vaults::VaultAction::SaveWallet(i));
        }
        let keys: Vec<usize> = self
            .session
            .keys
            .iter()
            .enumerate()
            .filter(|(_, k)| {
                let fp = k.master.fingerprint();
                fps.contains(&fp) || shares.contains(&fp)
            })
            .map(|(n, _)| n)
            .collect();
        for k in keys {
            let fp = self.session.keys[k].master.fingerprint();
            if !vault_screens::vault_has_key(self, v, fp) {
                self.vault_act(vaults::VaultAction::SaveKey(k));
            }
        }
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
                self.wallet = i;
                if let Some(c) = self.create.as_mut() {
                    c.built = Some(i);
                    c.error = None;
                    c.done[cstep::BUILD as usize] = true;
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

    fn restore_wallet(&mut self, name: &str, text: &str, source: &str) {
        match self.session.add_wallet(name, text, source) {
            Ok(i) => {
                self.wallet = i;
                if let Some(r) = self.restore.as_mut() {
                    r.wallet = Some(i);
                    r.error = None;
                    r.done[rstep::WALLET as usize] = true;
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
        let Some(scan) = self.scan.as_mut() else {
            return;
        };
        if scan.entry {
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
        if scan.part {
            let bytes = zeroize::Zeroizing::new(bytes);
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
        if scan.key {
            let mut bytes = zeroize::Zeroizing::new(bytes);
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
                    zeroize::Zeroize::zeroize(&mut self.entry.typed);
                    for (k, &i) in m.indices().iter().enumerate() {
                        if k > 0 {
                            self.entry.typed.push(' ');
                        }
                        self.entry.typed.push_str(words[usize::from(i)]);
                    }
                    self.scan = None;
                    self.sheet = None;
                    self.commands.push_back(Command::CameraOff);
                    // The words read are added as typed words, not
                    // through the word keyboard, which a small panel
                    // gets back if they are refused.
                    let keys = self.entry.keys.take();
                    self.entry_add();
                    if self.screen == Screen::Entry && keys.is_some() {
                        self.entry.keys = Some(forms::word_typer(lang));
                    }
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
        let Some((named, ext, data)) = self.read_code(bytes) else {
            return;
        };
        self.scanned += 1;
        let name = match named {
            Some(n) => self.free_inbox_name(&n),
            None => format!("scanned-{}.{ext}", self.scanned),
        };
        let item = Item::new(&name, data);
        if item.kind == FileKind::Other {
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
        self.toast(&format!("{name} is in Files"));
        if !matches!(
            self.screen,
            Screen::Spend | Screen::Visit | Screen::Restore | Screen::Home | Screen::Family
        ) {
            self.screen = Screen::Files;
        }
    }

    /// What one code read as a file holds: its name when it carries one,
    /// its extension and its bytes. `None` while a transfer in parts waits
    /// for more, and for a code that is refused, which leaves its reason
    /// as the scan's note.
    fn read_code(&mut self, bytes: Vec<u8>) -> Option<(Option<String>, &'static str, Vec<u8>)> {
        let scan = self.scan.as_mut()?;
        scan.reads += 1;
        let text = String::from_utf8_lossy(&bytes).trim().to_string();
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
            None if wallet::read_psbt(&bytes).is_some() => (None, "psbt", bytes),
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
        let addr = t
            .parse::<osk_bip::bitcoin::Address<osk_bip::bitcoin::address::NetworkUnchecked>>()
            .ok()?;
        let net = self.session.network();
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
        match self
            .session
            .add_mnemonic(&m, &self.entry.passphrase, &label, wanted)
        {
            Ok(fp) => {
                self.toast(&format!("Key {} added", fp_text(fp)));
                // A SeedQR scanned over the Spend tab or Restore returns
                // to it too.
                let back =
                    self.entry
                        .back
                        .or(matches!(self.screen, Screen::Family | Screen::Restore)
                            .then_some(self.screen));
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
        let seed = self.seed;
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
            if let Some(s) = self.spend.as_mut() {
                s.open = Some(wallet::step::NONCES);
                s.follow = true;
            }
            self.toast("This device's nonce is on the PSBT: carry it to the other signers");
            return;
        }
        self.refresh_spend();
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
            if self
                .session
                .sign(&mut psbt, self.seed, &mut s.musig)
                .is_ok()
            {
                s.spend.psbt = psbt;
            }
        }
        self.refresh_spend();
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
            if !(s.out_signed && s.out_tx) {
                self.act(Action::SignedToOutbox);
                self.act(Action::TxToOutbox);
            } else {
                self.screen = Screen::Files;
            }
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
        match self.screen {
            Screen::Family => self.vaults.focus.is_some() || self.seeds_typing(),
            Screen::Restore => self.seeds_typing(),
            Screen::Unlock | Screen::CreateVault | Screen::VaultContents => {
                self.vaults.focus.is_some()
            }
            Screen::Entry => true,
            Screen::Wallets => self.renaming.is_some(),
            Screen::Message => self.message.as_ref().is_some_and(|m| m.typing),
            _ => false,
        }
    }

    /// Empties the field typing goes to.
    fn clear_typing(&mut self) {
        use zeroize::Zeroize;
        match self.screen {
            Screen::Family | Screen::Restore if self.seeds_typing() => self.seeds_clear_typing(),
            Screen::Unlock | Screen::CreateVault | Screen::VaultContents | Screen::Family => {
                self.vault_clear_focused();
            }
            Screen::Entry if self.entry.on_passphrase => self.entry.passphrase.zeroize(),
            Screen::Entry => {
                self.entry.typed.zeroize();
                if self.entry.keys.is_some() {
                    self.entry.keys = Some(forms::word_typer(self.entry.language()));
                }
                self.entry.error = None;
            }
            Screen::Wallets => {
                if let Some(n) = self.renaming.as_mut() {
                    n.clear();
                }
            }
            Screen::Message => {
                if let Some(m) = self.message.as_mut() {
                    m.text.clear();
                }
            }
            _ => {}
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
        if self.potential_key(key) {
            return;
        }
        if self.catalog_key(key) {
            return;
        }
        // A selected field: Backspace empties it, a character replaces it,
        // any other key leaves it as it is.
        if std::mem::take(&mut self.select_all) && self.typing_field() {
            match key {
                KeyIn::Backspace => {
                    if !self.sticks.is_empty()
                        && matches!(
                            self.vaults.focus,
                            Some(vaults::Focus::Passphrase | vaults::Focus::Phrase(..))
                        )
                    {
                        // Nothing in a passphrase field moves with a stick in.
                    } else {
                        self.clear_typing();
                    }
                    return;
                }
                KeyIn::Char(c) if !c.is_control() => self.clear_typing(),
                _ => {}
            }
        }
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
                    return;
                }
                KeyIn::Backspace if b.checking => {
                    b.typed.pop();
                    return;
                }
                KeyIn::Down if b.open == Some(bstep::SEEDS) => {
                    b.pin += 1;
                    return;
                }
                KeyIn::Up if b.open == Some(bstep::SEEDS) => {
                    b.pin = b.pin.saturating_sub(1);
                    return;
                }
                _ => {}
            }
        }
        match key {
            KeyIn::Escape => {
                if self.sheet.is_some() && self.sheet != Some(Sheet::Lock) {
                    self.secret_cancel();
                    self.sheet = None;
                    self.qr = None;
                }
            }
            KeyIn::Down => self.glide(60.0),
            KeyIn::Up => self.glide(-60.0),
            _ => {}
        }
    }

    /// The offset of the region that scrolls now, in design units: the
    /// open sheet's, or the screen's.
    fn scroll_slot(&mut self) -> Option<&mut f32> {
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

    /// Runs `go` on the motion and the region that scrolls now, with how
    /// far it goes as the last frame drew it. A motion left over from
    /// another screen or sheet is stopped first.
    fn with_region<R>(
        &mut self,
        go: impl FnOnce(&mut motion::Motion, &mut motion::Region) -> R,
    ) -> Option<R> {
        let key = self.region_key();
        if self.motion_for != key {
            self.motion_for = key;
            self.motion.stop();
        }
        let extent = self.extent.filter(|(k, _)| *k == key).map(|(_, e)| e);
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
                // drag; it is a press until it moves.
                let key = self.region_key();
                // Not through a sheet that does not scroll: the page under
                // it is out of reach.
                let reachable = self.sheet.is_none() || key.1.is_some();
                self.drag = self
                    .extent
                    .filter(|(k, e)| reachable && *k == key && e.view.contains(x, y))
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
                if found.is_some_and(|(a, _)| a == Action::VisitBar) {
                    self.visit_drag(y);
                }
            }
            TouchPhase::Move => {
                if self.bar_held.is_some() {
                    self.bar_drag(y);
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                    return;
                }
                let held_bar = self.pressed.is_some_and(|(a, _)| a == Action::VisitBar);
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
                    && !held_bar
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
                if held_bar {
                    self.visit_drag(y);
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
                let was_selected = std::mem::take(&mut self.select_all);
                if let (Some((a, _)), Some((b, _))) = (pressed, found)
                    && a == b
                    && !vaults::is_hold(a)
                {
                    // A second press on the field typing goes to, or a
                    // drag across it, selects all of it.
                    let double = self
                        .last_tap
                        .is_some_and(|(l, t)| l == a && self.now_ms.saturating_sub(t) <= 450);
                    let dragged = (x - self.down_at.0).abs() > 12;
                    if (double || dragged) && field_action(a) && self.typing_field() {
                        self.select_all = !was_selected || dragged;
                        self.last_tap = None;
                        return;
                    }
                    self.last_tap = Some((a, self.now_ms));
                    self.act(a);
                    self.keygen_camera();
                    self.keygen_device();
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
        let key = self.region_key();
        // Not through a sheet that does not scroll.
        let reachable = self.sheet.is_none() || key.1.is_some();
        let (k, e) = self.extent?;
        if !reachable || k != key || e.own_bar || e.max <= 0.0 || !e.view.contains(x, y) {
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
        let key = self.region_key();
        let Some((_, e)) = self.extent.filter(|(k, _)| *k == key) else {
            return;
        };
        let g = ui::BarGeometry::of(e.view, e.max, self.f.max(0.1));
        let room = (g.track - g.thumb).max(1) as f32;
        let at = ((y - g.top - grab) as f32).clamp(0.0, room);
        if let Some(o) = self.scroll_slot() {
            *o = at / room * e.max;
        }
    }

    /// The stick's file list scrolled to where the scrollbar is held, at
    /// pixel row `y`: the thumb's middle under the finger.
    fn visit_drag(&mut self, y: i32) {
        let Some((top, h, thumb, max)) = self.visit.bar.get() else {
            return;
        };
        let room = (h - thumb).max(1) as f32;
        let at = ((y - top) as f32 - thumb as f32 / 2.0).clamp(0.0, room);
        self.list_offset = at / room * max;
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
        let offset = self.scroll_slot().map_or(0.0, |o| *o);
        let (seen_key, seen_offset, moved_at) = self.bar_seen;
        let moved_at = if seen_key != key {
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
        self.bar_seen = (key, offset, moved_at);
        let bar = motion::bar_alpha(moved_at, self.now_ms);
        // Drawn twice when a step card has just opened: the first time
        // finds out, the second draws it growing from closed.
        let mut follow_to = None;
        let mut scrolled = None;
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
                ui.select_all = self.select_all;
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
                ui.offset = offset;
                ui.compact = self.compact;
                ui.caret_on = self.caret_on();
                screens::draw(self, &mut ui);
                caret = ui.caret_drawn.then_some(ui.caret_on);
                scrolled = ui.scrolled;
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
        self.extent = scrolled.map(|s| (self.region_key(), s));
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
        self.bar_seen = (self.region_key(), offset, 0);
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
        // Leaving the new-key screen drops what it held.
        if self.screen != Screen::KeyGen && self.keygen.is_some() {
            self.keygen = None;
        }
        if self.screen != Screen::Bip85 {
            self.bip85 = None;
        }
        if self.screen != Screen::Explore {
            self.explore = None;
        }
        if self.screen != Screen::Tools {
            self.tools = None;
        }
        if self.sheet != Some(Sheet::WordList) {
            self.wordlist = None;
        }
        // Leaving the vanity screen stops a search and drops a find.
        if self.screen != Screen::Vanity {
            self.vanity = None;
        }
        if self.screen != Screen::Lightning && self.sheet != Some(Sheet::SecretOut) {
            self.lightning = None;
        }
        if self.screen != Screen::Silent && self.sheet != Some(Sheet::Qr) {
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
    v.extend([
        cstep::KEYS,
        cstep::BUILD,
        cstep::CHECK,
        cstep::VAULT,
        cstep::PUBLIC,
        cstep::BACKUP,
    ]);
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
            | Action::Rename
            | Action::MType
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
            Event::Scroll { dy, .. } => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.sheet = None;
                }
                // Pixels the content moves up, as the shell API states
                // them, in this layout's units.
                self.pan(f32::from(dy) / self.f.max(0.1));
                self.scroll_at = Some(self.now_ms);
            }
            Event::Wheel { dy, .. } => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.sheet = None;
                }
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
            Event::Scanned { bytes } => self.scanned(bytes),
            Event::CameraUnavailable => {
                if self.scan.take().is_some() {
                    self.sheet = None;
                    self.toast("No camera, or the camera was refused");
                }
            }
            _ => return,
        }
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
