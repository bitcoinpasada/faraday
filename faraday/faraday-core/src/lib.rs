//! Faraday's app: the screens, the Wallets tab's session, the Inbox and the
//! Outbox, and the stick rule (`docs/history/PLAN.md` §5).
//!
//! It is an [`faraday_shell_api::App`], so the shells drive it with events and
//! read its frame. Removable storage is the one thing that interface has
//! no event for, so this crate adds a second channel beside it: the shell
//! reports the sticks it sees and the files read from them through
//! [`Faraday::storage`], and takes the reads and writes this app asks for
//! from [`Faraday::poll_storage`]. The app itself does no I/O.
//!
//! Keys are typed, scanned or loaded from a vault for the session only;
//! PSBTs and wallets move by stick or QR. Test wallets come in on a test
//! stick (`testkit`, built only with the `testkit` feature).
//!
//! # Code map
//!
//! The `mod` lines below are grouped by area, and every module is named
//! here (`tools/lint-layout.sh` fails when one is not):
//!
//! - The shell's protocol and the app's types: `storage_api` (sticks and
//!   storage events), `version`, `build` (a build ID described and
//!   compared, F5), `action` (the `Action` enum), `qr_view`
//!   (the QR view and the camera's scan state), `receipt` and `state` (the
//!   state of Entry, Message and the stick visit). Each other area's state
//!   is in its own `mod.rs` or file: `DecodeState` in `decode`,
//!   `SpendState` in `wallet`, `BackupState` in `wallet_backup`,
//!   `CreateState` in `create`, `RestoreState` in `restore`. `lib.rs`
//!   re-exports them all, so a caller names `faraday_core::Action`.
//! - State and actions: `work` (costly work counted), `compute` (where
//!   event code gets its token), `flow`, `inputs`, `edit` (the caret and
//!   the selection), `motion` (scrolling), `out` (what the core
//!   hands the shell: commands, storage commands, the toast), `pointer` (the
//!   hits, the press, the drag, the hover), `idle` (the idle lock's timers), `fade` (a
//!   change fading in), `menu` (the ⋮ menus), `nav` (the navigation by
//!   window class, with `nav/screen.rs`), `file_menu`, `in_progress`,
//!   `compact` (the small panel), `panes` (list and detail), `medium`
//!   (what the removable medium is called), `memory` (what is kept across
//!   a lock), `copy_check` (a copy by hand checked), `set_time`, `stick`
//!   (`pills.rs`, `settings.rs`, `wipe.rs`), `visit_taken`, `trail` (where
//!   Back lands: the screens left, as they were seen), and `ui` (drawing
//!   in design units).
//! - Screens and drawing: `screens` (the frame, the sidebar, the sheets and
//!   the screens with no area folder), `drawing`, each area's `screen.rs`
//!   (`vault/screen.rs`, `keygen/screen.rs`), `glance` (a wallet at a
//!   glance), `guide` (Guided mode's walk-through), `scan_key`, `scan_read`
//!   (what a code read is), `pdf` (the
//!   printed sheets), `picture` (a QR code to keep), `demo` (what Practice
//!   refuses), `devices` (what else captures the screen or the keys) and
//!   `edid` (what a screen says it is).
//! - Flows around a key, seed, words or passphrase: `keygen`, `codex32` (with `codex32/screen.rs`, its checksum worksheet, `codex32/worksheet_screen.rs`, and its dice and the booklet's dice sheet, `codex32/dice_screen.rs`, and the entry's input panel, `codex32/input_panel.rs`; New key's dice in the input panel, `keygen/dice_panel.rs`), `seeds`
//!   (with `seeds/read.rs`), `wallet` (with `wallet/nonce.rs`, `wallet/account.rs`), `inbox`, `create`, `create_from`,
//!   `bip85`, `forms`, `fresh`, `gpg`, `secure_boot`, `lightning`,
//!   `silent`, `vanity`, `grind`, `paper`, `quiz`, `secrets`,
//!   `word_entry`, `vault` (with `vault/load.rs` and `vault/out.rs`),
//!   `chart_edit`, `boot_import`, `backup_flow/wallet.rs` and `codex32_correct` (§8.4's correction, with its `screen.rs`). They hold
//!   handles and public results; key use is `faraday-keys` (F70,
//!   `docs/KEYS.md`), and `tools/lint-secrets.sh` rule 4 holds it.
//! - Wallets and PSBTs: `plan`, `decode`, `family`, `explore`, `catalog`, `register`, `review`.
//! - Backups and the backup flow: `backup_flow`, `backups`, `board`,
//!   `held` (the backup before spending), `restore`, `wallet_backup`,
//!   `bstep` (the checklist's items, numbered), `codex32_backup` (its
//!   codex32 item and the set's actions, with its `screen.rs`).
//! - Vaults: `codes` (TOTP codes); `vault` is listed with the flows above.
//! - Sticks, boot and files: `transfer` (the desktop as the
//!   device's QR link), `names` (the Starter's install names),
//!   `secure_boot/state.rs`.
//! - Faraday Demo's host: `host` (the strip at the window's foot, the
//!   preview frame and the pointer carried into the app, with
//!   `host/screen.rs` and its lists under a finger in `host/scroll.rs`; `docs/DESKTOP-HOST.md`).
//! - Support: `slow` (the presses whose work waits for a frame), `wordlist`, `learn`, `modes` (F81, with `modes/screen.rs`), `samples` (with `samples/screen.rs`, and `samples/put.rs`, Use a sample, whose buttons Faraday Demo's strip shows), `tools`, `testkit`.
//!
//! `Faraday::act_inner` is the dispatch for presses: it runs the checks that
//! come before any action, then sends each `Action` to its area's
//! `<area>_act` (`create_act` in `create`, `restore_act` in `restore`,
//! `backup_act` in `wallet_backup`, `wallet_act` in `wallet`, `visit_act` in
//! `visit_taken` and the like). Only navigation, Back, the settings and the
//! Learn sheet are handled in `act_inner` itself.
//!
//! Every `Screen` variant and the file that draws it. An area draws in its
//! own `screen.rs`; a screen with no area folder is in `screens/`, whose
//! `mod.rs` holds the frame and the dispatch, `sidebar.rs` the sidebar, `sheets.rs` the sheets,
//! `learn_pane.rs` Learn's supporting pane and `parts.rs` what two or more screens share.
//!
//! | `Screen` | Drawn in |
//! |---|---|
//! | `Home` | `screens/home.rs` |
//! | `WalletsStart` | `screens/wallets_start.rs` |
//! | `Wallets` | `screens/wallets.rs` |
//! | `Spend` | `screens/spend.rs` |
//! | `Files` | `screens/files.rs` |
//! | `Visit` | `screens/visit.rs` |
//! | `Entry` | `screens/entry.rs` |
//! | `Backup` | `screens/backup.rs` |
//! | `Message` | `screens/message.rs` |
//! | `CheckMessage` | `screens/check_message.rs` |
//! | `Create` | `screens/create.rs` |
//! | `Restore` | `screens/restore.rs` |
//! | `Settings` | `screens/settings.rs` |
//! | `SetTime` | `screens/set_time.rs` |
//! | `Randomness` | `screens/randomness.rs` |
//! | `Vaults` | `vault/screen.rs` |
//! | `Backups` | `backups/screen.rs` |
//! | `CreateVault` | `vault/screen.rs` |
//! | `KeyGen` | `keygen/screen.rs` |
//! | `Bip85` | `bip85/screen.rs` |
//! | `Silent` | `silent/screen.rs` |
//! | `Explore` | `explore/screen.rs` |
//! | `Lightning` | `lightning/screen.rs` |
//! | `Tools` | `tools/screen.rs` |
//! | `Unlock` | `vault/screen.rs` |
//! | `VaultContents` | `vault/screen.rs` |
//! | `Codes` | `codes/screen.rs` |
//! | `Family` | `family/screen.rs` |
//! | `Vanity` | `vanity/screen.rs` |
//! | `Decode` | `screens/decode.rs` |
//! | `NonceCheck` | `screens/nonce_check.rs` |
//! | `Catalog` | `screens/catalog.rs` |
//! | `Transfer` | `transfer/screen.rs` |
//! | `BackupFlow` | `backup_flow/screen.rs` |

// The shell's protocol and the app's types: storage, version, actions, QR view, receipt, state.
mod action;
pub mod build;
mod qr_view;
mod receipt;
mod state;
mod storage_api;
mod version;

// State and actions: the app's state, its events and what a press does.
mod compact;
mod compute;
pub mod copy_check;
mod edit;
mod fade;
pub mod file_menu;
pub mod flow;
mod idle;
pub mod in_progress;
pub mod inputs;
pub mod medium;
pub mod memory;
mod menu;
pub mod motion;
pub mod nav;
mod out;
mod panes;
mod pointer;
pub mod set_time;
pub mod stick;
mod trail;
pub mod ui;
pub mod visit_taken;
pub mod work;

// Screens and drawing: what is drawn from the state (each area's `screen.rs` is in its folder).
pub mod demo;
pub mod devices;
mod drawing;
pub mod edid;
pub mod glance;
mod guide;
pub mod pdf;
pub mod picture;
mod scan_key;
pub mod scan_read;
mod screens;

// Flows around a key, seed, words or passphrase; key use is faraday-keys (F70).
pub mod bip85;
pub mod boot_import;
pub mod chart_edit;
pub mod codex32;
pub mod codex32_correct;
pub mod create;
mod create_from;
pub mod forms;
pub mod fresh;
pub mod gpg;
pub mod grind;
pub mod inbox;
pub mod keygen;
pub mod lightning;
pub mod paper;
pub mod quiz;
mod review;
pub mod secrets;
pub mod secure_boot;
pub mod seeds;
pub mod silent;
pub mod vanity;
pub mod vault;
pub mod wallet;
pub mod word_entry;

// Wallets and PSBTs.
pub mod catalog;
pub mod decode;
pub mod explore;
pub mod family;
pub mod plan;
mod register;
mod slow;

// Backups and the backup flow.
pub mod backup_flow;
pub mod backups;
pub mod board;
mod held;
pub mod restore;
pub mod wallet_backup;

// Vaults.
pub mod codes;

// Sticks, boot and files.
pub mod names;
pub mod transfer;

// Support: word lists, the Learn pages, the tools page, test data.
pub mod host;
pub mod learn;
pub mod modes;
pub mod samples;
#[cfg(feature = "testkit")]
pub mod testkit;
pub mod tools;
pub mod wordlist;

pub use action::Action;
pub use compact::OskPress;
pub use create::CreateState;
pub use decode::DecodeState;
#[doc(hidden)]
pub use drawing::View;
pub use fade::FadeCounts;
/// Text a person types that may be a secret (`faraday_intake`'s).
pub use faraday_intake::secret_buf as secret_text;
/// Faraday's wording, in its own crate.
pub use faraday_strings as strings;
pub use file_menu::Unrenamed;
pub use host::{Host, HostMenu, HostPress};
pub use medium::Medium;
pub use modes::{DESKTOP_MODES, Mode, STICK_MODES};
pub use qr_view::{
    Ask, Code, NoPicture, QR_PARTS, QR_SPEEDS, QrFormat, QrSource, QrView, SEEN_FRAMES,
    ScanPurpose, ScanState, Seen, TOO_FINE_TENTHS,
};
pub(crate) use qr_view::{TOO_FINE_PASSES, UrFed, send_error, transfer_name, ur_feed, ur_note};
pub(crate) use receipt::sha256_of;
pub use receipt::{Receipt, ReceiptFile};
pub use restore::RestoreState;
pub use screens::qr_code;
pub use secure_boot::state::SecureBootAtStart;
pub use state::{EntryState, MessageState, VisitState};
pub use stick::pills::{BOOT_FOLDER, FIRST_BACKUP, Pill, SticksWanted, backup_folder};
pub use storage_api::{StickInfo, StorageCommand, StorageEvent};
pub use version::{
    BUILD, Program, VERSION, is_release, parse_version_output, version_label, version_line,
};
pub use wallet::SpendState;
pub use wallet_backup::BackupState;

use crate::strings::{EN, fill};
use faraday_intake::{arrival, seed_in_code, seedqr_words};
use std::collections::BTreeSet;

use faraday_crypto::work::Compute;
use faraday_draw::Canvas;
use faraday_shell_api::{App, Command, DisplayInfo, Event, Frame, Key as KeyIn, TouchPhase};

use wallet::{FileKind, KeyTexts, Session, fp_text};

/// Minutes without input before the session locks, at first (F58).
pub const IDLE_LOCK_MIN: u16 = 30;
/// Minutes without input before the machine powers off, at first,
/// counted from the last input across the lock (F58).
pub const IDLE_OFF_MIN: u16 = 60;
/// How long before an idle lock its warning comes up, at most: half the
/// lock time when that is shorter.
pub const IDLE_WARN_MS: u64 = 5 * 60_000;

/// The exit code that asks the app loop for a fresh process (`docs/history/PLAN.md` §5.3).
pub const RESTART_CODE: u8 = 75;

/// The exit code that asks init to restart the computer instead of
/// powering it off (`docs/FLOWS.md`, Restart; F42). 75 stays the lock's.
pub const RESTART_COMPUTER_CODE: u8 = 76;

/// The screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Home.
    Home,
    /// The Wallets tab's first page: every flow, by what the person has.
    WalletsStart,
    /// The Wallets tab: the wallets, the wallet card, and every flow (`docs/WALLETS.md`).
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
    /// Setting the time by hand (`docs/TOTP.md` §3).
    SetTime,
    /// Add randomness (`docs/OWN-RANDOMNESS.md` §2.2).
    Randomness,
    /// The vaults the Inbox and Outbox hold.
    Vaults,
    /// Every wallet known and where its backup is (`docs/history/SIMPLIFY.md` §5.1).
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
    /// The calculators.
    Tools,
    /// Unlocking one slot of a vault.
    Unlock,
    /// An open vault's contents.
    VaultContents,
    /// TOTP codes for the entries in open vaults (`docs/TOTP.md` §5).
    Codes,
    /// The Spend tab (`docs/SPEND-FROM-BACKUP.md`).
    Family,
    /// A vanity address for a loaded key.
    Vanity,
    /// A transaction taken apart.
    Decode,
    /// A signed transaction brought in, checked rather than signed.
    NonceCheck,
    /// Tools: every flow on one page, with its standards.
    Catalog,
    /// The online app as the device's QR link: files out as codes, in by
    /// camera ([`transfer`]). Only when [`Faraday::online`].
    Transfer,
    /// The shared backup flow: where a key will be kept, its sticks, its
    /// vaults, the key, the lock and the writes (`docs/BACKUP-FLOW.md`).
    BackupFlow,
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

/// The largest file read from a stick, as the disk process reads them
/// (`faraday-files` `MAX_READ`): a larger one is listed and not read.
pub const READ_MAX: u64 = 18 * 1024 * 1024;

/// A file named as Faraday names a seed it writes out: its words, its SeedQR, a BIP-85 child.
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
    if stick::settings::is_file(name) {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "psbt" => Some(EN.stick_kind_psbt),
        "txt" | "json" | "descriptor" => Some(EN.stick_kind_text),
        "txn" => Some(EN.stick_kind_transaction),
        "faraday-sheet" => Some(EN.stick_kind_print_sheet),
        "fvlt" => Some(EN.stick_kind_vault),
        "efi" => Some(EN.stick_kind_efi_image),
        "png" => Some(EN.stick_kind_qr_codes),
        _ => Some(EN.stick_kind_file),
    }
}

/// A file in the Inbox or the Outbox.
#[derive(Clone)]
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
    /// SHA-256 of a vault's or an EFI image's bytes, taken once
    /// when the file came in (each is public: a vault's file is sealed);
    /// `None` for any other file. An Item's bytes never change after it
    /// is made.
    pub sha: Option<[u8; 32]>,
    /// The wallet a wallet file describes, read once when the file came
    /// in, as [`Item::sha`] is taken (`docs/SPEED.md` §3.3): `None` for
    /// any other file. An Item's bytes are never changed after it is
    /// made, so this stays what they say; nothing reads the bytes again for it.
    pub wallet: Option<faraday_bip::policy::WalletPolicy>,
    /// What a PSBT file says (its txid, its signatures, what its first
    /// input spends), read once when the file came in, as
    /// [`Item::wallet`] is: `None` for any other file. Which loaded
    /// wallet it spends from is found from it
    /// ([`wallet::Session::spends_from`]), so it follows the wallets.
    pub psbt: Option<wallet::PsbtSeen>,
}

/// Prints the name, the byte count and the `secret` flag, never the bytes:
/// a file can be a secret.
impl std::fmt::Debug for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Item))
            .field("name", &self.name)
            .field("len", &self.bytes.len())
            .field("secret", &self.secret)
            .finish_non_exhaustive()
    }
}

/// An Inbox or Outbox file may be a seed or a share written in the clear:
/// its bytes are wiped when it goes, whatever it holds.
impl Drop for Item {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.bytes);
    }
}

impl Item {
    /// Costs a classification, a parse of a wallet file's description and a
    /// hash of a vault or an EFI image. The file as it came in,
    /// classified, with what it describes read once.
    fn new(c: &Compute, name: &str, bytes: Vec<u8>) -> Item {
        let wallet::Classified { kind, psbt, wallet } = wallet::classify_read(c, name, &bytes);
        let sha =
            matches!(kind, FileKind::Vault | FileKind::EfiImage).then(|| sha256_of(c, &bytes));
        Item {
            name: name.to_string(),
            bytes,
            kind,
            secret: false,
            picture: None,
            sha,
            wallet,
            psbt,
        }
    }

    /// What a PSBT file says, read when it came in ([`Item::psbt`]): a
    /// lookup.
    pub fn psbt(&self) -> Option<&wallet::PsbtSeen> {
        self.psbt.as_ref().filter(|_| self.kind == FileKind::Psbt)
    }

    /// The wallet a wallet file describes, read when it came in. A lookup.
    pub fn wallet(&self) -> Option<&faraday_bip::policy::WalletPolicy> {
        self.wallet.as_ref()
    }

    /// Costs a hash of a whole file (through `sha256_of`). SHA-256 of its
    /// bytes: the one taken when it came in ([`Item::sha`], a
    /// vault's or an EFI image's), a lookup. Any other file is hashed now.
    pub fn hash(&self, c: &Compute) -> [u8; 32] {
        self.sha.unwrap_or_else(|| sha256_of(c, &self.bytes))
    }

    /// What it holds: a picture's codes, or the file itself.
    pub fn holds(&self) -> FileKind {
        self.picture.unwrap_or(self.kind)
    }

    /// A lock drops it from the Inbox: it may hold a secret, which the
    /// next process must not inherit (`docs/history/PLAN.md` §5.3).
    pub fn dropped_at_lock(&self) -> bool {
        self.holds().may_be_secret() || self.secret
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
            FileKind::Entries | FileKind::SecretWallet => Some(secrets::Ack::Any),
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
    /// not saved; or, with no seed to seal with, what is lost.
    LockAsk,
    /// Power off with files in the Outbox.
    Power,
    /// A QR code, or an animated run of them.
    Qr,
    /// The camera, reading QR codes.
    Scan,
    /// Locked for idleness: what waits in the Outbox.
    Locked,
    /// A new input device, waiting to be believed or ignored.
    NewInput,
    /// The Learn pages for the screen on show.
    Learn,
    /// A word list, every word with its number, over the screen it was opened from.
    WordList,
    /// A secret on its way out: into a vault, or the Outbox after a warning.
    SecretOut,
    /// Writing the Outbox to a stick, asked for while a secret is held: the lock it takes first.
    WriteOut,
    /// The idle lock coming: a countdown, and what the lock does.
    IdleWarn,
    /// A seed from the Inbox becoming a wallet: a passphrase or none, and its kind.
    Potential,
    /// What the boot stick brought: remove the stick, unlock its vaults, choose what to import.
    Import,
    /// Something pressed that loads a key, with a stick attached: it
    /// carries on once the stick is pulled ([`Faraday::pull`]).
    Pull,
    /// A node or a line of the wallet's chart: what it is, where, and its
    /// actions (`docs/history/NEW-WALLET.md` §9).
    Chart,
    /// The backup board's list, opened by **Add** in the narrow column
    /// (`docs/BACKUP-FLOW.md` §17.12 item 3).
    BoardList,
    /// A drop let a place see the balance it could not before
    /// (`docs/BACKUP-FLOW.md` §17.5): **Keep it** or **Take it off**.
    BoardSees,
    /// Partial descriptions added (`docs/BACKUP-FLOW.md` §17.19 item 4):
    /// **Take it off every place** or **Keep it**.
    BoardPartials,
    /// A chip's ⋮ menu on the backup board (`docs/BACKUP-FLOW.md`
    /// §17.20): a popup under the ⋮ on `wide`, a sheet on the small panel.
    BoardMenu,
    /// A seed the camera read, asking how it loads: with no passphrase or with one (`seed_read`).
    SeedRead,
    /// **Sweep a private key**: a WIF's addresses, its PSBT reviewed and signed (`seeds::sweep`).
    Sweep,
    /// Delete files: chosen files, the free space or the whole files
    /// partition, asked first (`stick_wipe`).
    Delete,
    /// Zeroing under way: how far, and Stop.
    Zeroing,
    /// A stick visit's write would go over files already on the stick:
    /// Replace, Keep both or Cancel (`visit_taken`).
    Taken,
    /// What is attached that can capture the screen or the keys, each
    /// named with what it can do, and before a secret the reminders
    /// (`devices`).
    Devices,
    /// A wallet a key not of its own can spend alone by the key path,
    /// waiting to be added: the danger line naming that key, Cancel, and
    /// a hold that adds it (F132).
    KeyPath,
    /// **Which account to load?**: a Coldcard export of a single-sig and a silent payments account.
    Account,
    /// The wallet a spend's PSBT defines, to register with a hold (F135).
    Register,
    /// A vault record's Save to a stick: its files, to tick, and Save
    /// (`docs/BACKUP-FLOW.md` §17.7).
    VaultSave,
    /// Cancel pressed in a flow whose ending loses something: what is
    /// lost, **Cancel the flow** and **Keep going** (F51).
    CancelFlow(in_progress::Going),
    /// The same in a key flow opened from Make a new wallet (`docs/FLOWS.md`, Cancel).
    CancelKey,
    /// A file row's ⋮ menu (`docs/FLOWS.md` decision 61): a popup under
    /// the ⋮ on `wide`, a sheet on the small panel.
    FileMenu,
    /// **Rename {name}**: a file For the stick, or one on the stick visited.
    Rename,
    /// A file's Details: name, kind, size and SHA-256.
    Details,
    /// **Load from {vault}**, after an unlock from the boot sheet or the
    /// Vaults tab: what to load, ticked (`vault_load`, F62).
    UnlockLoad,
    /// **Open this copy?**: an unlock held at an older copy (`vault::OlderHold`).
    OlderCopy,
    /// **Record it as a child seed?**: BIP-85 child words held (F166).
    ChildRecord,
    /// **A BIP-85 child seed**: a wallet made over one recorded (F166).
    ChildNotice,
    /// **Seeds with no passphrase stored**, from a wallet short of a key:
    /// one to load with a passphrase typed now, checked against the
    /// wallet (`vault_load`, F62).
    PassphraseSeeds,
    /// **Sign with {key}**, from Sign a file's **Sign**: the files chosen,
    /// the time the signatures will carry with **Set the time**, and the
    /// hold that signs (F63).
    GpgSign,
    /// **Mode**, from the pill or Settings' Mode row: the modes this app
    /// offers, the one in use marked (F81, `modes/screen.rs`).
    Mode,
    /// **Load samples**, from the Practice pill's neighbour (`samples/screen.rs`).
    Samples,
    /// A top app bar's ⋮ menu: the compact window's, or a bar's overflow.
    NavMenu(Option<nav::Overflow>),
    /// New key's Length card's Other: the menu of 15, 18 and 21 words (`keygen/screen.rs`).
    KeyLengths,
    /// **Restore a GPG key**: the public keys in Files.
    GpgRestore,
}

pub(crate) use picture::{picture_codes, picture_names, picture_payloads};

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

/// An account key of a loaded wallet held here, as
/// [`Faraday::wallet_key`] finds it.
pub struct WalletKey {
    /// The master fingerprint, as text.
    pub fp: String,
    /// `[fingerprint/path]xpub`, as the wallet's descriptor writes it.
    pub text: String,
    /// What kind of wallet it is a key of, by name.
    pub kind: String,
    /// The kind, when the key is at Create's own account for it.
    pub standard: Option<create::NewKind>,
    /// The seed's place among the session's keys.
    pub key: usize,
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

/// The vault part of a new wallet's backup, asked before the keys exist
/// (`docs/BACKUP-FLOW.md` §10.2): whether vaults are part of it, which is
/// answered by the person and not before, and how many.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VaultAsk {
    /// Yes or no, once answered.
    pub answer: Option<bool>,
    /// How many vaults will be part of the backup: one or more.
    pub count: usize,
}

impl Default for VaultAsk {
    fn default() -> VaultAsk {
        VaultAsk {
            answer: None,
            count: 1,
        }
    }
}

/// The most vaults a backup is set up with at once; a count past it is
/// not a count a person means.
const MAX_VAULTS: usize = 1024;

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
    /// No card now: Check's **Back up wallet** opens Back up.
    pub const BACKUP: u8 = 4;
    /// Whether vaults are part of the backup, before Keys.
    pub const WHERE: u8 = 5;
    /// Bring in the sticks of the vaults, before Keys.
    pub const STICKS: u8 = 6;
    /// Unlock and make the vaults, before Keys.
    pub const VAULTS: u8 = 7;
    /// How many vaults, before the sticks.
    pub const HOW_MANY: u8 = 8;
    /// How many cards there are.
    pub const COUNT: usize = 9;
}

/// What Restore says of a seed that is not one of the description's keys.
pub const NOT_A_KEY: &str = EN.app_not_wallet_key;

/// The restore cards, in order (`docs/history/NEW-WALLET.md` §12.1).
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
    /// Whether vaults are part of the backup, before Seeds.
    pub const WHERE: u8 = 6;
    /// How many vaults, before Seeds.
    pub const HOW_MANY: u8 = 7;
    /// How many cards.
    pub const COUNT: u8 = 8;
}

/// The backup cards, in order.
pub mod bstep;
pub mod codex32_backup;

/// The backup's two parts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BStage {
    /// The board and the plan's cards.
    #[default]
    Plan,
    /// The checklist made from the plan.
    Checklist,
}

/// The cards of the backup's plan, in order (`docs/BACKUP-FLOW.md`
/// §15.1).
pub mod qstep {
    /// The watch-only software and the files' form.
    pub const SOFTWARE: u8 = 4;
    /// The plan: the map, approved with Make the checklist.
    pub const PLAN: u8 = 6;
    /// The board: the places, what each keeps, and the list of what the
    /// backup can keep (`docs/BACKUP-FLOW.md` §17).
    pub const BOARD: u8 = 8;
    /// The most places a plan has.
    pub const MAX_PLACES: usize = 240;
}

/// A press on a plan's cards (`docs/BACKUP-FLOW.md` §15.1), the same on
/// Back up and in a wallet flow's own plan card: Back up gives each its
/// own [`Action`], the flow its [`backup_flow::steps::FlowAction::Plan`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanPress {
    /// Open or close a card, by [`qstep`].
    Q(u8),
    /// The card's Continue: the card after it.
    Next(u8),
    /// A row of the Software card, by [`qrow`].
    Answer(u8, u8),
    /// The keys left off each share.
    Omit(u8),
}

impl PlanPress {
    /// The press as Back up's own action.
    pub fn backup(self) -> Action {
        match self {
            PlanPress::Q(q) => Action::BQ(q),
            PlanPress::Next(q) => Action::BQNext(q),
            PlanPress::Answer(l, r) => Action::BAnswer(l, r),
            PlanPress::Omit(n) => Action::BOmit(usize::from(n)),
        }
    }
}

/// Costs what `of` costs. The cards of a plan, in
/// order (`docs/BACKUP-FLOW.md` §15.1, §17.1): the board, the software
/// where the description goes into software or as files, and The plan.
/// Back up's and a wallet flow's alike (§19.12 item 1).
pub fn plan_cards(c: &Compute, shape: &plan::Shape, a: &plan::Answers) -> Vec<u8> {
    plan_cards_on(&plan::Mapped::of(c, shape, a))
}

/// [`plan_cards`] on the plan's map `m`.
pub(crate) fn plan_cards_on(m: &plan::Mapped) -> Vec<u8> {
    let mut v = vec![qstep::BOARD];
    if m.asks_software() {
        v.push(qstep::SOFTWARE);
    }
    v.push(qstep::PLAN);
    v
}

/// Costs what `read_wallet` costs. Whether record `r` is a wallet share
/// (type 13) of the wallet with checksum `cs`, told by its field 5,
/// never by its name; with `j`, share `j` (field 3, from 1) of it
/// (`docs/BACKUP-FLOW.md` §15.6).
pub(crate) fn share_record_is(
    c: &Compute,
    r: faraday_keys::vault::RecordView<'_>,
    cs: &str,
    j: Option<usize>,
) -> bool {
    use faraday_vault::records::{field, kind};
    r.kind() == kind::WALLET_SHARE
        && r.public_text(field::SHARE_WALLET)
            .and_then(|t| wallet::read_wallet(c, t).ok())
            .is_some_and(|p| p.checksum() == cs)
        && j.is_none_or(|j| {
            r.public(field::SHARE_NUMBER)
                .and_then(|b| b.first())
                .is_some_and(|&n| usize::from(n) == j + 1)
        })
}

/// [`plan_cards`] for a plan drawn by `plan_cards`.
pub(crate) fn plan_cards_of(ctx: &plan::cards::PlanCtx) -> Vec<u8> {
    plan_cards_on(&ctx.mapped)
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

/// A write under way: the stick's label, the stick's identity (its FAT
/// volume serial, else where the shell found it), each file sent with its
/// SHA-256, and whether an answer has started its receipt.
pub type Writing = (String, String, Vec<(String, [u8; 32])>, bool);

/// Where a stick visit column's scrollbar was drawn, in pixels: the
/// track's top and height, the thumb's height, and the furthest the list
/// scrolls in design units.
pub type VisitBarAt = (i32, i32, i32, f32);

/// The app.
pub struct Faraday {
    canvas: Option<Canvas>,
    f: f32,
    w: f32,
    h: f32,
    /// The display is a small panel: under 600 dp wide, laid out one
    /// column, pages in place of the sidebar (`Faraday::is_compact`).
    compact: bool,
    pointer: pointer::Pointer,
    out: out::Out,
    /// The second of the clock last drawn on a screen that shows it live
    /// (Now, a TOTP code and its seconds left), so a tick that moves it
    /// on draws again.
    clock_drawn: Option<u64>,
    restart: bool,
    /// The exit asked for a restart of the computer, not a power-off.
    restart_computer: bool,
    last_display: Option<DisplayInfo>,
    /// What the person chose: scale, theme, motion, idle times, QR speed,
    /// time zone and guided mode (`stick/settings.rs`).
    settings: stick::settings::Settings,
    /// Setting the time by hand (`docs/TOTP.md` §3).
    set_time: set_time::SetTimeArea,
    /// The idle lock's timers and what its warning shows.
    idle: idle::Idle,
    /// A secret has been held or typed in this process. Only a fresh
    /// process is clean (`docs/history/PLAN.md` §5.1).
    tainted: bool,
    /// New input devices waiting to be believed, and those kept out
    /// (`docs/history/PLAN.md` §4.6).
    inputs: inputs::InputsArea,
    /// What a frame works out once (`docs/SPEED.md` §3.3): on only while
    /// `render` runs.
    drawing: drawing::Drawing,
    /// What is worked out once and kept by what it is worked out from
    /// (`drawing/kept.rs`).
    kept: drawing::Kept,
    /// The vaults: the open slots and the vault screens' state, and the
    /// item's scroll.
    vaults: vault::VaultArea,
    /// What is kept across a lock besides the settings: the signed amounts,
    /// whether they are sealed into a vault, the wallets checked.
    memory: memory::MemoryArea,
    /// The wallet the Backups screen is on, and what this power-on saw of
    /// each wallet's backup (`docs/history/SIMPLIFY.md` §5): kept across a
    /// lock, gone at power-off.
    backups_area: backups::BackupsArea,
    /// The session's seed and the pool of fresh answers (`fresh.rs`).
    fresh: fresh::Fresh,
    now_ms: u64,

    /// The screen shown.
    pub screen: Screen,
    /// A sheet over it.
    pub sheet: Option<Sheet>,
    /// The open sheet's offset as the frame draws it.
    sheet_offset: f32,
    /// A small panel's docked keyboard and walk-through.
    panel: compact::CompactArea,
    /// The Wallets tab's session.
    pub session: Session,
    /// The transaction being decoded.
    decode: decode::DecodeArea,
    /// Files read from sticks.
    inbox: inbox::InboxArea,
    /// Results waiting for a stick.
    pub outbox: Vec<Item>,
    /// The sticks present and what is chosen about them.
    stick: stick::StickArea,
    /// Seed entry.
    entry: state::EntryArea,
    /// The Wallets tab: the wallet chosen, the new name being typed, the
    /// card's scroll and the transaction being signed.
    wallets: wallet::WalletsArea,
    /// The stick visit: its screen, the pull waiting, the receipt, Files' row
    /// menus and the For the stick list's scroll.
    visit: visit_taken::VisitArea,
    /// What the boot stick brought at its first look this power-on,
    /// waiting to be imported (`boot_import`).
    import: boot_import::BootImportArea,
    /// On a small panel, the height of the pinned bar the last frame
    /// drew: the page is laid out above it.
    pub(crate) pin_h: std::cell::Cell<f32>,
    /// Pixels scrolled past in the long list on screen (the stick's
    /// files, Files, the wallet list): continuous, not row-snapped, so a
    /// wheel notch moves the content by its own pixels like everywhere
    /// else.
    pub list_offset: f32,
    /// The wallet's chart: its page, its open sheet, the key drawn bold and
    /// what the sheets hold.
    glance: glance::GlanceArea,
    /// What scrolls: the motion, the regions the last frame drew, the
    /// scrollbar's memory, the sheet's offset and the page's height
    /// (`motion.rs`, `docs/MOTION.md` §3.3).
    scroll: motion::Scroll,
    /// The QR sheet's contents.
    qr: qr_view::QrArea,
    /// The Spend tab.
    family: family::FamilyArea,
    /// A vanity address being searched for.
    vanity: vanity::VanityArea,
    /// The wallet backup checklist (`Screen::Backup`), while it is open.
    backup: wallet_backup::WalletBackupArea,
    /// Signing a message.
    message: state::MessageArea,
    /// Creating a wallet.
    create: create::CreateArea,
    /// Restoring a wallet.
    restore: restore::RestoreArea,
    /// New key: the key being made, the keys made this session, the
    /// camera and the shell's randomness source.
    keygen: keygen::KeyGenArea,
    /// The Learn sheet and the steps ticked on its pages.
    learn: learn::LearnArea,
    /// The word-list sheet, while it is open.
    wordlist: wordlist::WordListArea,
    /// BIP-85 being derived.
    bip85: bip85::Bip85Area,
    /// Silent payments on show.
    silent: silent::SilentArea,
    /// Explore on show.
    explore: explore::ExploreArea,
    /// The Lightning node key on show.
    lightning: lightning::LightningArea,
    /// Tools on show.
    tools: tools::ToolsArea,
    /// A secret waiting for the person to say where it goes.
    secrets: secrets::SecretsArea,
    /// The camera sheet, the cameras the shell can open and the one
    /// chosen.
    camera: scan_key::Camera,
    /// Transfer, in the online app.
    transfer: transfer::TransferArea,
    /// The caret, the selection, Shift, where the last frame drew the
    /// fields' characters and the caret's blink.
    typing: edit::TypingArea,
    /// The trail Back follows (F77), the mouse's forward button and the
    /// way back the last frame drew.
    trail: trail::Trail,
    /// A seed the camera read, waiting on [`Sheet::SeedRead`].
    seed_read: seeds::SeedsArea,
    /// A press whose work waits for a frame (`docs/SPEED.md` §9), and
    /// whether a frame has been drawn since it was pressed.
    later: Option<Later>,
    later_drawn: bool,
    /// The step column's open card, and one opening or closing.
    disclosed: Option<Disclosed>,
    /// A change of screen or sheet fading in, how fast this machine has
    /// lately shown a fade's steps, the frosted page under the open
    /// sheet, and the screen the last frame drew (`fade.rs`).
    fade: fade::Fade,
    /// The Guided switch's pill sliding: where it started, 0 on Steps
    /// only and 1 on Guided, and the tick it started at.
    guided_moving: Option<(f32, Option<u64>)>,
    /// What the start-up self-test found (`faraday_selftest`): run when the
    /// display first arrives, before any input. A failure blocks the app
    /// on a screen naming the check, with Exit its only control.
    selftest: Option<faraday_selftest::Outcome>,
    /// Running on an online machine (the desktop app); sheets become PDFs.
    pub online: bool,
    /// A seed typed here is on an online computer: the seed line is drawn.
    pub seed_line: bool,
    /// Held by another app that shows only Transfer and the Learn sheet
    /// (the Starter, `docs/STARTER.md` §2), set with [`Faraday::online`]:
    /// no sidebar, no Home and no way back drawn: the holder's top app bar
    /// is above the frame (§3.1d); leaving Transfer or a sheet ends it.
    pub held: bool,
    /// The shell lets a person insert and pull sticks by hand, and the app
    /// draws a pill for each ([`Faraday::stick_pills`]): the desktop app
    /// and the website demo. Never set on the device.
    pub shell_sticks: bool,
    /// The shell answers `RequestClipboard` with the clipboard's text: the
    /// desktop app. The scanner's Paste is dimmed where it is not set,
    /// which is never on the device (`docs/QR.md` §8 item 5).
    pub clipboard: bool,
    /// What the removable medium is called and drawn as: a stick, or an
    /// SD card on the Pi. The shell sets it at start.
    pub medium: Medium,
    /// Whether the machine started with Secure Boot on, as the stick shell
    /// read it once at start. Unknown everywhere else.
    pub secure_boot: SecureBootAtStart,
    /// The computer's maker (`sys_vendor`), as the stick shell read it.
    pub maker: String,
    /// The computer's product family (`product_family`), as the stick
    /// shell read it.
    pub maker_family: String,
    /// The computer's model (`product_name`), as the stick shell read it.
    pub maker_model: String,
    /// What is attached that can capture the screen or the keys, as the
    /// stick shell read it (`devices`).
    pub devices: devices::DeviceCheck,
    /// The mode a lock in progress ends in; cleared when it is not gone through.
    next_mode: modes::ModesArea,
    /// The samples Practice offers, set by the desktop; none elsewhere.
    pub samples: Option<&'static samples::Samples>,
    /// The Load samples sheet's rows (`samples::SheetState`).
    sample_area: samples::SamplesArea,
    /// The backup flows, at most one on screen (`docs/BACKUP-FLOW.md` §7),
    /// the keys made here held until their backup is done, and the
    /// board's list scroll.
    flows: backup_flow::BackupFlowArea,
    /// The flows begun, in the order they were begun, each with the screen
    /// of its own it was last on (`docs/FLOWS.md`, Flows in progress):
    /// only the order and the way back, worked out after each press.
    in_progress: in_progress::InProgress,
}

impl Default for Faraday {
    fn default() -> Self {
        Self::new()
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
            pointer: pointer::Pointer::new(),
            out: out::Out::new(),
            clock_drawn: None,
            restart: false,
            restart_computer: false,
            last_display: None,
            settings: stick::settings::Settings::default(),
            set_time: set_time::SetTimeArea::default(),
            idle: idle::Idle::new(),
            tainted: false,
            inputs: inputs::InputsArea::default(),
            drawing: drawing::Drawing::default(),
            kept: drawing::Kept::default(),
            vaults: vault::VaultArea::default(),
            memory: memory::MemoryArea::default(),
            backups_area: backups::BackupsArea::default(),
            fresh: fresh::Fresh::default(),
            now_ms: 0,
            screen: Screen::Home,
            sheet: None,
            sheet_offset: 0.0,
            panel: compact::CompactArea::default(),
            session: Session::default(),
            decode: decode::DecodeArea::default(),
            inbox: inbox::InboxArea::default(),
            stick: stick::StickArea::default(),
            outbox: Vec::new(),
            entry: state::EntryArea::default(),
            wallets: wallet::WalletsArea::default(),
            visit: visit_taken::VisitArea::default(),
            import: boot_import::BootImportArea::default(),
            pin_h: std::cell::Cell::new(0.0),
            list_offset: 0.0,
            glance: glance::GlanceArea::default(),
            scroll: motion::Scroll::default(),
            qr: qr_view::QrArea::default(),
            family: family::FamilyArea::default(),
            vanity: vanity::VanityArea::default(),
            backup: wallet_backup::WalletBackupArea::default(),
            message: state::MessageArea::default(),
            create: create::CreateArea::default(),
            restore: restore::RestoreArea::default(),
            keygen: keygen::KeyGenArea::default(),
            learn: learn::LearnArea::default(),
            wordlist: wordlist::WordListArea::default(),
            bip85: bip85::Bip85Area::default(),
            silent: silent::SilentArea::default(),
            explore: explore::ExploreArea::default(),
            lightning: lightning::LightningArea::default(),
            tools: tools::ToolsArea::default(),
            secrets: secrets::SecretsArea::default(),
            camera: scan_key::Camera::default(),
            transfer: transfer::TransferArea::default(),
            typing: edit::TypingArea::new(),
            trail: trail::Trail::default(),
            seed_read: seeds::SeedsArea::default(),
            later: None,
            later_drawn: false,
            disclosed: None,
            fade: fade::Fade::default(),
            guided_moving: None,
            selftest: None,
            online: false,
            seed_line: false,
            held: false,
            shell_sticks: false,
            clipboard: false,
            medium: Medium::Stick,
            secure_boot: SecureBootAtStart::Unknown,
            maker: String::new(),
            maker_family: String::new(),
            maker_model: String::new(),
            devices: devices::DeviceCheck::default(),
            next_mode: modes::ModesArea::default(),
            samples: None,
            sample_area: samples::SamplesArea::default(),
            flows: backup_flow::BackupFlowArea::default(),
            in_progress: in_progress::InProgress::default(),
        }
    }

    /// The camera the person chose since the last call, by the id
    /// [`StorageEvent::Cameras`] gave it, for the shell to open.
    pub fn take_camera(&mut self) -> Option<String> {
        self.camera.take_change()
    }

    /// Whether the last exit asked for a fresh process.
    pub fn restart_requested(&self) -> bool {
        self.restart
    }

    /// Whether the exit asks for the computer to restart rather than
    /// power off (`docs/FLOWS.md`, Restart): the shell ends the app with
    /// [`RESTART_COMPUTER_CODE`].
    pub fn computer_restart_requested(&self) -> bool {
        self.restart_computer
    }

    /// Whether the power sheet offers Restart: the PC stick and the Pi,
    /// not Faraday Demo, an app on someone's own system.
    pub fn offers_restart(&self) -> bool {
        !self.shell_sticks
    }

    /// Whether a storage request is waiting for the shell (a lookup).
    pub fn storage_pending(&self) -> bool {
        !self.out.storage_out.is_empty()
    }

    /// The screen's name, and the sheet's when one is up, for a shell's
    /// slow-frame line: `Backup` or `Backup / Qr` (a small string).
    pub fn screen_name(&self) -> String {
        match self.sheet {
            Some(sheet) => format!("{:?} / {:?}", self.screen, sheet), // not shown: a log line
            None => format!("{:?}", self.screen),                      // not shown: a log line
        }
    }

    /// The next storage request, or `None`.
    pub fn poll_storage(&mut self) -> Option<StorageCommand> {
        self.out.storage_out.pop_front()
    }

    /// Delivers what the shell knows about storage.
    pub fn storage(&mut self, event: StorageEvent) {
        let screen = self.screen;
        self.note_here();
        self.storage_event(event.in_own_words());
        self.backup_settle();
        self.stick.note_visit(self.screen == Screen::Visit);
        if self.screen != screen {
            // A stick that takes the person to another screen leaves
            // this one as a press would.
            self.after_input();
        } else {
            self.trail.forget_entered();
            // A device still waiting comes back up once no other sheet is.
            self.input_sheet();
            self.device_sheet();
        }
    }

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn storage_event(&mut self, event: StorageEvent) {
        let c = self.compute();
        let event = match self
            .settings_event(event)
            .and_then(|e| self.import_event(e))
        {
            Some(e) => e,
            None => {
                self.out.dirty = true;
                self.out.commands.push_back(Command::Draw);
                return;
            }
        };
        match event {
            StorageEvent::Sticks(sticks) => {
                self.sticks_changed(sticks);
                self.zero_sticks_changed();
            }
            StorageEvent::Read { stick, name, bytes } if self.flow_wants_read(&stick, &name) => {
                self.flow_read_answer(&name, Some(bytes));
            }
            // A picture the write step wrote, read back for its codes.
            StorageEvent::QrRead {
                name,
                bytes,
                payloads,
            } if self.flow_wants_qr(&name) => {
                self.flow_qr_answer(&name, Some((bytes, payloads)));
            }
            StorageEvent::ReadFailed { name, .. } if self.flow_wants_qr(&name) => {
                self.flow_qr_answer(&name, None);
            }
            StorageEvent::ReadFailed { stick, name, .. } if self.flow_wants_read(&stick, &name) => {
                self.flow_read_answer(&name, None);
            }
            StorageEvent::StickSpace { stick, free, size } if self.flow_wants_space(&stick) => {
                self.flow_space_answer(free, size);
            }
            StorageEvent::StickProblem { stick, read_only } => {
                self.flow_stick_problem(&stick, read_only);
            }
            StorageEvent::Written {
                stick,
                name,
                wrote_as,
            } if self.flow_wants_written(&stick, &name) => {
                self.flow_write_answer(&name, Ok(wrote_as));
            }
            StorageEvent::WriteFailed {
                stick,
                name,
                reason,
            } if self.flow_wants_written(&stick, &name) => {
                self.flow_write_answer(&name, Err(reason));
            }
            StorageEvent::Read { stick, name, bytes } => {
                // A file Faraday reads as nothing else still comes in, as
                // a File: to sign, or to send as codes.
                let kept = self.inbox_name_for(&name, &bytes);
                let item = Item::new(&c, &kept, bytes);
                if item.kind == FileKind::Vault
                    && let Some(s) = self.stick.iter().find(|s| s.id == stick)
                {
                    let label = s.label.clone();
                    self.vault_read_from(&item.name, &item.bytes, &label);
                }
                // The visit's rows say what came in; only a seed waiting
                // for the stick's removal is worth a line.
                let line = (item.kind == FileKind::Words
                    && self.visit.load_after.contains(&item.name))
                .then(|| {
                    fill(
                        EN.storage_event_copied_words,
                        &[&item.name.to_string(), self.medium.noun()],
                    )
                });
                self.inbox.push(item);
                if let Some(line) = line {
                    self.visit.log.push((line, true));
                }
                self.visit.inn.remove(&name);
                self.visit.reading.remove(&name);
                self.save_boxes();
            }
            StorageEvent::ReadFailed { name, reason, .. } => {
                self.visit.reading.remove(&name);
                self.visit.log.push((format!("{name}: {reason}"), false));
            }
            StorageEvent::Written { name, wrote_as, .. } => {
                self.receipt_add(&name, None);
                // A rename is carried by one write, to the stick written:
                // another stick keeps its own name for the same vault.
                self.visit.files_menu_mut().renamed.remove(&name);
                // A copy from the Inbox stays there, for the next stick.
                if self.visit.writing_inbox.remove(&name) {
                    self.visit.from_inbox.remove(&name);
                } else {
                    // Written, it leaves For the stick for the receipt. A
                    // vault stays in Files as the stick now holds it, to
                    // unlock again; its copy from before goes.
                    if let Some(k) = self.outbox.iter().position(|i| i.name == name) {
                        let item = self.outbox.remove(k);
                        self.inbox_vault_written(&c, &item);
                    }
                    self.visit.out.remove(&name);
                }
                let line = if wrote_as == name {
                    fill(EN.storage_event_wrote_read, &[&name.to_string()])
                } else {
                    fill(
                        EN.storage_event_wrote,
                        &[&name.to_string(), &wrote_as.to_string()],
                    )
                };
                self.visit.log.push((line, true));
                self.save_boxes();
            }
            StorageEvent::WriteFailed { name, reason, .. } => {
                self.visit_write_failed(&name, &reason)
            }
            StorageEvent::NotKept => self.not_kept(),
            StorageEvent::Wiped { stick, name } => self.wiped(&stick, &name, None),
            StorageEvent::Renamed {
                stick,
                from,
                to,
                refused,
            } => self.file_renamed(&stick, &from, &to, refused),
            StorageEvent::Hashed { stick, name, sha } => self.file_hashed(&stick, &name, sha),
            StorageEvent::StickSpace { free, size, .. } => self.zero_space(free, size),
            StorageEvent::Zeroing {
                next, total, done, ..
            } => self.zero_step(next, total, done),
            StorageEvent::ZeroFailed { stick, reason } => {
                // The room on a stick could not be asked for: the flow's
                // reading of it ends, with the stick unusable.
                self.flow_space_failed(&stick);
                self.zero_failed(&reason)
            }
            StorageEvent::WipeFailed {
                stick,
                name,
                reason,
            } => self.wiped(&stick, &name, Some(&reason)),
            StorageEvent::Printed { path } => {
                self.toast(&fill(EN.storage_event_saved, &[&path.to_string()]))
            }
            StorageEvent::Clock { unix_secs } => {
                self.vaults.set_clock(unix_secs, self.now_ms);
                return;
            }
            StorageEvent::Memory { available_mib } => {
                self.vaults.set_memory_free(available_mib);
                if !matches!(self.screen, Screen::CreateVault | Screen::Unlock) {
                    return;
                }
            }
            StorageEvent::Cameras(list) => {
                // Sent as the shell looks again: drawn again only when it
                // changes, since what the copy item offers depends on it.
                if !self.camera.set_cameras(list) {
                    return;
                }
            }
            StorageEvent::QrSeen {
                width,
                height,
                corners,
                module_tenths,
                read,
            } => {
                let now_ms = self.now_ms;
                if let Some(s) = self.camera.scan_mut() {
                    let now = s.frames;
                    let away = s
                        .seen
                        .is_none_or(|(_, at)| now.wrapping_sub(at) > SEEN_FRAMES);
                    s.seen = Some((
                        Seen {
                            width,
                            height,
                            corners,
                            read,
                        },
                        now,
                    ));
                    let fine = module_tenths < TOO_FINE_TENTHS;
                    if !read && fine {
                        s.too_fine += 1;
                    } else {
                        s.too_fine = 0;
                    }
                    if s.too_fine >= TOO_FINE_PASSES {
                        s.note = Some(EN.storage_event_too_fine.to_string());
                    }
                    // Large enough and still not read after a while: the
                    // picture is the problem (`docs/QR.md` §8 item 8).
                    if read || fine {
                        s.unread_since = None;
                    } else {
                        match s.unread_since {
                            Some(since) if !away => {
                                if now_ms.saturating_sub(since)
                                    >= faraday_draw::tokens::POOR_PICTURE_MS
                                {
                                    s.note = Some(EN.scan_hold_square_hint.to_string());
                                }
                            }
                            _ => s.unread_since = Some(now_ms),
                        }
                    }
                }
            }
            StorageEvent::QrRead {
                name,
                bytes,
                payloads,
            } => {
                self.visit.inn.remove(&name);
                self.visit.reading.remove(&name);
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
                let mut item = Item::new(&c, &got, bytes);
                item.picture = Some(holds);
                self.inbox.push(item);
                self.save_boxes();
            }
            StorageEvent::PrintFailed { reason } => {
                self.toast(&fill(EN.storage_event_not_printed, &[&reason.to_string()]))
            }
            StorageEvent::HostFiles { dir, files, opens } => {
                if !self.transfer_listed(dir, files, opens) {
                    return;
                }
            }
            StorageEvent::Hovered(over) => self.transfer_hovered(over),
            StorageEvent::Dropped { path, size } => self.transfer_dropped(path, size),
            StorageEvent::HostRead { path, bytes } => self.transfer_read(path, bytes),
            StorageEvent::HostReadFailed { path, reason } => {
                self.transfer_read_failed(path, reason);
            }
            StorageEvent::Saved { path } => self.transfer_saved(Ok(path)),
            StorageEvent::SaveFailed { reason } => self.transfer_saved(Err(reason)),
            e @ (StorageEvent::NewInput { .. }
            | StorageEvent::InputTyped { .. }
            | StorageEvent::InputTapped { .. }
            | StorageEvent::InputGone { .. }) => self.input_event(e),
            StorageEvent::Restored {
                inbox,
                outbox,
                kept,
            } => {
                self.inbox.replace(
                    inbox
                        .into_iter()
                        .map(|(n, b)| Item::new(&c, &n, b))
                        .collect(),
                );
                self.outbox = outbox
                    .into_iter()
                    .map(|(n, b)| Item::new(&c, &n, b))
                    .collect();
                self.restore_kept(&kept);
            }
        }
        self.out.dirty = true;
        self.out.commands.push_back(Command::Draw);
    }

    /// Whether anything secret is being typed: seed words, a BIP-39 or
    /// vault passphrase, a prompt's line. A secret sheet for an Inbox file
    /// on its way to another stick does not count: it holds a copy of
    /// what the Inbox already holds, and the stick it goes to must stay
    /// open (`docs/history/PLAN.md` §5.4).
    fn secret_typed(&self) -> bool {
        self.keygen.state().is_some()
            || self.secrets.out().is_some_and(|o| !o.to_visit)
            || self.lightning.state().is_some()
            || !self.entry.state().typed.is_empty()
            || !self.entry.state().passphrase.is_empty()
            || !self.vaults.passphrase.text.is_empty()
            || self
                .inbox
                .potential()
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

    /// The clean state of `docs/history/PLAN.md` §5.1: this process has never held or
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

    /// [`Faraday::clean`] without noting anything: whether this process has
    /// held or typed no secret, as the screen draws it.
    pub(crate) fn clean_now(&self) -> bool {
        !self.tainted && !self.camera_on() && !self.holds_secret() && !self.secret_typed()
    }

    /// The camera is on, reading codes.
    ///
    /// Transfer's Receive, in the online app, does not count: that app is
    /// never clean of the network, and no stick waits on its camera.
    fn camera_on(&self) -> bool {
        let transfer = self
            .camera
            .scan()
            .is_some_and(|s| s.purpose == ScanPurpose::Transfer);
        (self.sheet == Some(Sheet::Scan) && !transfer) || self.keygen.camera_on()
    }

    /// A stick held back while the camera was on arrives once it is off.
    fn release_held_stick(&mut self) {
        if !self.stick.held() || self.camera_on() {
            return;
        }
        self.stick.set_held(false);
        if !self.stick.is_empty() {
            self.stick_arrived();
            self.out.dirty = true;
            self.out.commands.push_back(Command::Draw);
        }
    }

    /// Input arrived: the idle timers start again.
    fn input_now(&mut self) {
        self.idle.input(self.now_ms);
    }

    /// The idle time this process locked after, when it locked for
    /// idleness.
    pub(crate) fn idle_locked_after(&self) -> Option<u64> {
        self.idle.locking()
    }

    /// This process follows a lock for idleness after `ms` without input:
    /// the time counts toward power-off, and the lock screen shows.
    pub(crate) fn idle_locked(&mut self, ms: u64) {
        self.idle.followed_lock(ms);
        self.sheet = Some(Sheet::Locked);
    }

    /// Whether locking would wipe anything: a key, a vault, a wallet or a
    /// spend.
    fn lockable(&self) -> bool {
        self.holds_secret()
            || !self.vaults.open.is_empty()
            || !self.session.wallets.is_empty()
            || self.wallets.spend().is_some()
    }

    /// Time without input so far, counted across an idle lock.
    pub(crate) fn idle_ms(&self) -> u64 {
        self.idle.ms(self.now_ms)
    }

    /// When the idle warning comes up and when the lock follows, ms
    /// without input; `None` when the session never locks for idleness.
    pub(crate) fn idle_lock_at(&self) -> Option<(u64, u64)> {
        (self.idle_lock_min() > 0).then(|| {
            let lock = u64::from(self.idle_lock_min()) * 60_000;
            (lock - IDLE_WARN_MS.min(lock / 2), lock)
        })
    }

    /// Ms without input at which the machine powers off, when it would:
    /// nothing held, nothing in the Outbox, not the online app.
    pub(crate) fn idle_off_at(&self) -> Option<u64> {
        (self.idle_off_min() > 0
            && !self.online
            && !self.lockable()
            && self.outbox.is_empty()
            && !self.flow_screen())
        .then(|| u64::from(self.idle_off_min()) * 60_000)
    }

    /// The idle timers (`docs/history/PLAN.md` §12.3). Returns whether the screen
    /// changed.
    fn idle_tick(&mut self, now_ms: u64) -> bool {
        let c = self.compute();
        let idle = self.idle.ms_starting(now_ms);
        let min = 60_000u64;
        // The warning, with its countdown, before the lock.
        // Make a new wallet before its keys are kept anywhere: no idle
        // lock at all (`docs/history/NEW-WALLET.md` §18).
        let held = self.lock_held(&c);
        if let Some((warn, lock)) = self.idle_lock_at()
            && idle >= warn
            && idle < lock
            && self.lockable()
            && self.idle.locking().is_none()
            && !held
        {
            let changed = self.sheet != Some(Sheet::IdleWarn);
            if changed && self.sheet.is_none() {
                self.sheet = Some(Sheet::IdleWarn);
                self.idle.set_warn_min(idle / min);
                self.explore_hide();
            }
            let left = (lock - idle).div_ceil(1000);
            if self.sheet == Some(Sheet::IdleWarn) && self.idle.shown() != Some(left) {
                self.idle.set_shown(Some(left));
                return true;
            }
            return false;
        }
        // The Locked sheet counts down to power-off.
        if self.sheet == Some(Sheet::Locked)
            && let Some(off) = self.idle_off_at()
        {
            let left = off.saturating_sub(idle).div_ceil(1000);
            if self.idle.shown() != Some(left) {
                self.idle.set_shown(Some(left));
                if idle < off {
                    return true;
                }
            }
        }
        if self.idle_lock_min() > 0
            && idle >= u64::from(self.idle_lock_min()) * min
            && self.lockable()
            && self.idle.locking().is_none()
            && !held
        {
            self.idle.set_locking(Some(idle));
            // No one is there to ask: the lock goes ahead, and an open
            // vault's changes with no seed to seal them are lost, as the
            // warning before it said.
            self.lock_now();
            return true;
        }
        if self.idle_off_min() > 0
            && idle >= u64::from(self.idle_off_min()) * min
            && !self.online
            && !self.lockable()
            && !self.flow_screen()
        {
            if self.outbox.is_empty() {
                self.out.commands.push_back(Command::Exit);
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
        let had = !self.stick.is_empty();
        let ids_before: Vec<String> = self.stick.iter().map(|s| s.id.clone()).collect();
        self.stick.set_sticks(sticks);
        // The keys follow: no way of adding one adds it with a stick in.
        let noun = (!self.stick.is_empty()).then(|| self.medium.noun());
        self.session.keys.set_stick_attached(noun);
        let present: Vec<String> = self.stick.iter().map(|s| s.id.clone()).collect();
        // What the shell found unusable comes with each listing: a stick
        // that no longer reports a problem has none.
        let _ = &present;
        self.stick.clear_problems();
        // The medium the device started from goes by what it is, not by its volume label.
        let boot = self.medium.boot();
        for s in self.stick.iter_mut().filter(|s| s.boot) {
            s.label = boot.to_string();
        }
        let ids_now: Vec<String> = self.stick.iter().map(|s| s.id.clone()).collect();
        if self.visit.stick >= self.stick.len() {
            self.visit.stick = 0;
        }
        let names: BTreeSet<String> = self
            .stick
            .get(self.visit.stick)
            .map(|s| s.files.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default();
        self.visit.inn.retain(|n| names.contains(n));
        self.flow_sticks_changed();
        if self.stick.is_empty() {
            self.stick.set_held(false);
            self.stick.set_not_now(false);
            if self.sheet == Some(Sheet::Lock) {
                self.sheet = None;
            }
            // A passphrase waiting on the stick takes typing now.
            if self.screen == Screen::Unlock
                && self.vaults.focus.is_none()
                && self.vault_files().iter().any(|f| f.open.is_none())
            {
                self.vaults.set_focus(Some(vault::Focus::Passphrase));
            }
            if self.screen == Screen::Visit {
                // Where the visit's chip said it goes (§4.7).
                let unlock = self
                    .visit
                    .after_visit()
                    .is_none()
                    .then(|| self.visit_unlocks())
                    .flatten();
                self.screen = self.visit.after_visit_mut().take().unwrap_or(Screen::Home);
                if self.screen == Screen::Family {
                    self.family_settle();
                }
                if let Some(i) = unlock {
                    self.vault_act(vault::VaultAction::Open(i));
                }
            }
            if had {
                let loaded = self.load_after_pull() + self.import_load_after_pull();
                let said = if loaded == 0 {
                    fill(EN.sticks_changed_removed_only, &[self.medium.cap()])
                } else {
                    fill(
                        EN.sticks_changed_removed,
                        &[
                            self.medium.cap(),
                            &loaded.to_string(),
                            (if loaded == 1 {
                                EN.sticks_changed_key
                            } else {
                                EN.sticks_changed_keys
                            }),
                        ],
                    )
                };
                self.toast(&said);
            }
            self.import_stick_gone();
            // What was pressed with the stick in carries on, and the
            // reminders the sheet listed are seen.
            if self.sheet == Some(Sheet::Pull) {
                self.sheet = None;
                if let Some(a) = self.visit.pull_mut().take() {
                    if self.devices.held.is_empty() {
                        self.devices.understand();
                    }
                    self.act(a);
                }
            }
            return;
        }
        self.import_stick_gone();
        if ids_now == ids_before {
            self.flow_sticks_changed();
            self.release_held_stick();
            return;
        }
        // A stick arriving while the camera is on is held back, and
        // arrives when the camera is off (`docs/history/PLAN.md` §5.4): then a clean
        // process visits it and any other asks to lock first. The
        // scanner is never replaced with its camera still on.
        if self.camera_on() {
            self.stick.set_held(true);
            return;
        }
        self.stick_arrived();
    }

    /// A stick arrived, or one held back is taken now.
    fn stick_arrived(&mut self) {
        let c = self.compute();
        // A stick arrived. Only a clean process takes it; any other locks
        // first (`docs/history/PLAN.md` §5.4). No passphrase field takes typing while
        // it is attached.
        if matches!(
            self.vaults.focus,
            Some(vault::Focus::Passphrase | vault::Focus::Phrase(..) | vault::Focus::Dice)
        ) {
            self.vaults.set_focus(None);
        }
        // The backup flow on screen takes the stick: never the visit, at any
        // step (`docs/BACKUP-FLOW.md` §5).
        if self.flow_stick_arrived() {
            return;
        }
        self.visit.log.clear();
        self.visit.settings = None;
        // The first look at the boot stick this power-on: what it holds is
        // copied into memory for the import (`boot_import`).
        let first_look = self.stick.settings().is_none();
        let boot = self.stick.iter().find(|s| s.boot).cloned();
        if !self.clean() {
            self.stick.set_not_now(false);
            self.sheet = Some(Sheet::Lock);
        } else {
            self.visit.out = self.visit_default_out(&c);
            self.visit.inn = self.visit_default_in();
            if self.sheet == Some(Sheet::Import) {
                self.sheet = None;
            }
            // The boot stick alone at its first look, or back for its
            // import, leaves Home on screen (`boot_import`).
            if !self.import_keeps_home(first_look, boot.as_ref()) {
                // A stick arriving in the middle of Create or Restore
                // brings the person back there when it is pulled.
                if matches!(
                    self.screen,
                    Screen::Create | Screen::Restore | Screen::Family
                ) {
                    *self.visit.after_visit_mut() = Some(self.screen);
                }
                self.screen = Screen::Visit;
            }
            if first_look
                && self.import.state().is_none()
                && let Some(b) = boot.as_ref()
                && b.files.iter().any(|(n, _)| !stick::settings::is_file(n))
            {
                self.import_start(b);
            }
            // The boot stick's settings, once a session (`stick_settings`).
            if self.stick.settings().is_none()
                && let Some(boot) = self.stick.iter().find(|s| s.boot)
            {
                match boot.files.iter().find(|(n, _)| stick::settings::is_file(n)) {
                    Some((n, _)) => self.out.storage_out.push_back(StorageCommand::Read {
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
        self.out.storage_out.push_back(StorageCommand::SaveBoxes {
            inbox: pack(&self.inbox),
            outbox: pack(&self.outbox),
            kept,
        });
    }

    fn toast(&mut self, text: &str) {
        self.out.toast(text, self.now_ms);
    }

    /// How much of the toast shows, 0 to 1, and how far below its place
    /// it is, in units: it rises in, and fades as its time runs out.
    pub(crate) fn toast_shown(&self) -> (f32, f32) {
        self.out.toast_shown(self.now_ms, self.reduce_motion())
    }

    /// Whether loading a key is allowed now: only once the self-test has
    /// passed, and never with a stick attached.
    pub fn may_load_keys(&self) -> bool {
        self.selftest_passed() && self.stick.is_empty()
    }

    /// What `action` does, named for the sheet that asks for the stick to
    /// be pulled first, when it loads a key or turns the camera on (a
    /// frame may picture a seed); `None` for anything a stick may stay in
    /// for.
    pub fn pull_what(&self, action: Action) -> Option<&'static str> {
        use backup_flow::steps::FlowAction as F;
        use catalog::Go;
        Some(match action {
            Action::Scan
            | Action::ScanFor(_)
            | Action::BScan
            | Action::Vault(vault::VaultAction::ScanEntry)
            | Action::Family(family::FamilyAction::Scan | family::FamilyAction::Again(true)) => {
                EN.pull_what_scan
            }
            Action::Entry(_)
            | Action::EntryWallet(_)
            | Action::ScanSeed
            | Action::ScanKey(_)
            | Action::ScanPart
            | Action::LoadKey(_)
            | Action::KeyGen(None)
            | Action::CSlotVault(..)
            | Action::Vault(vault::VaultAction::PassphraseKey(_)) => EN.pull_what_add_a_key,
            Action::KeyGen(Some(_)) | Action::KeyGenSlip39 => EN.pull_what_make_a_key,
            Action::PotentialOpen(_) => EN.pull_what_load_wallet,
            // The ticks stay on the Load samples state while the sheet
            // waits; the load runs when the stick is out.
            Action::SamplesLoad if self.sample_area.any_ticked() => EN.pull_what_load_samples,
            // Unlocking for another flow: the Pull sheet asks first, and
            // Unlock opens once the stick is out.
            Action::Vault(vault::VaultAction::OpenFrom(i, _))
                if self.vault_files().get(i).is_some_and(|f| f.open.is_none()) =>
            {
                EN.pull_what_unlock
            }
            Action::Vault(vault::VaultAction::ListFrom(_)) => EN.pull_what_unlock,
            Action::Flow(F::UnlockVault(_) | F::UnlockDone(_)) => EN.pull_what_unlock,
            Action::Flow(F::CreateVault(_)) => EN.pull_what_create_vault,
            Action::Flow(F::Make | F::CopyKey | F::TypeString) => EN.pull_what_make_a_key,
            Action::Catalog(i) => match catalog::TILES.get(usize::from(i))?.go {
                Go::VaultCategory(c)
                    if self.vaults.open.is_empty()
                        && !self.vault_files().is_empty()
                        && backup_flow::Kind::of_category(c).is_none() =>
                {
                    EN.pull_what_unlock
                }
                Go::NewShares => EN.pull_what_make_a_key,
                Go::NewKey | Go::AddKey(_) | Go::SeedQr | Go::Sweep => EN.pull_what_add_a_key,
                // Sign and Decode with nothing to open scan: the Scan
                // they press waits instead.
                Go::CheckAddress | Go::Scan => EN.pull_what_scan,
                _ => return None,
            },
            _ => return None,
        })
    }

    /// What the start-up self-test found, once it has run: how many
    /// checks passed, or the name of the first that failed.
    pub fn selftest(&self) -> Option<faraday_selftest::Outcome> {
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
    /// [`faraday_selftest::CHECKS`]: for a test that a failed check blocks the
    /// app.
    #[doc(hidden)]
    pub fn run_selftest_with(&mut self, checks: &[faraday_selftest::Check]) {
        let c = self.compute();
        self.selftest = Some(faraday_selftest::run_checks(&c, checks));
        if self.selftest_failed() {
            self.sheet = None;
        }
        self.out.dirty = true;
    }

    /// The toast shown, if one is current.
    pub fn toast_text(&self) -> Option<&str> {
        self.out.toast.as_ref().map(|(t, _)| t.as_str())
    }

    /// A public or sealed file to the Outbox. A secret never comes this
    /// way: it goes through [`Faraday::offer_secret`], which asks first.
    fn put_outbox(&mut self, name: &str, bytes: Vec<u8>) {
        let c = self.compute();
        let item = Item::new(&c, name, bytes);
        if item.exposure() == secrets::Exposure::Secret {
            debug_assert!(false, "a secret reached put_outbox: {name}");
            return;
        }
        self.outbox.retain(|i| i.name != name);
        self.outbox.push(item);
        self.save_boxes();
    }

    /// Costs a hash of a whole file (through `sha256_of`). What a write's
    /// record keeps of a file written with this SHA-256: the hash, or
    /// nothing (all zeros) for a secret waiting in the Outbox or the Inbox,
    /// since a receipt and a flow's sticks are kept across a lock and a
    /// hash of a seed's words or a passphrase is derived from it. Hashes
    /// each file there that is a secret.
    pub(crate) fn hash_to_keep(&self, c: &Compute, hash: [u8; 32]) -> [u8; 32] {
        let secret =
            self.outbox.iter().chain(self.inbox.iter()).any(|i| {
                i.exposure() == secrets::Exposure::Secret && sha256_of(c, &i.bytes) == hash
            });
        if secret { [0; 32] } else { hash }
    }

    /// A write's answer onto the receipt: the first answer of a write
    /// replaces the last receipt with a new one (`docs/history/SIMPLIFY.md` §4.3).
    fn receipt_add(&mut self, name: &str, failure: Option<String>) {
        let c = self.compute();
        let now = self.clock_now();
        let Some((label, stick, sent, started)) = self.visit.writing.as_mut() else {
            return;
        };
        let Some(k) = sent.iter().position(|(n, _)| n == name) else {
            return;
        };
        let (name, hash) = sent.remove(k);
        let stick = stick.clone();
        let opened = (!*started).then(|| {
            *started = true;
            Receipt {
                label: label.clone(),
                stick: stick.clone(),
                time: now,
                files: Vec::new(),
                by_flow: false,
            }
        });
        if sent.is_empty() {
            self.visit.writing = None;
        }
        if let Some(r) = opened {
            *self.visit.receipt_mut() = Some(r);
        }
        let hash = self.hash_to_keep(&c, hash);
        if let Some(r) = self.visit.receipt_mut().as_mut() {
            r.files.push(ReceiptFile {
                name,
                hash,
                verified: failure.is_none(),
                failure,
            });
        }
    }

    /// The vault, by place in [`Faraday::vault_files`], a stick visit
    /// unlocks once the stick is pulled: a locked one sealed for it, still
    /// For the stick or written by the last write.
    fn visit_unlocks(&self) -> Option<usize> {
        self.vault_files().iter().position(|f| {
            f.open.is_none()
                && (f.in_outbox
                    || self
                        .visit
                        .receipt()
                        .as_ref()
                        .is_some_and(|r| r.wrote(&f.name)))
        })
    }

    /// The stick visit's chip (`docs/history/SIMPLIFY.md` §4.7): where pulling
    /// the stick goes. Back to the flow the stick came in during, Unlock
    /// when a vault was sealed for the visit, else Home.
    pub fn visit_then(&self) -> String {
        match *self.visit.after_visit() {
            Some(s) => fill(EN.visit_then_then, &[self.back_name(s)]),
            None if self.visit_unlocks().is_some() => EN.visit_then_then_unlock.to_string(),
            None => EN.visit_then_then_home.to_string(),
        }
    }

    /// Remembers that vault file `name`, these bytes, was read from the
    /// stick labelled `label`.
    pub(crate) fn vault_read_from(&mut self, name: &str, bytes: &[u8], label: &str) {
        let c = self.compute();
        self.visit.vault_from_mut().retain(|(n, ..)| n != name);
        self.visit.vault_from_mut().push((
            name.to_string(),
            sha256_of(&c, bytes),
            label.to_string(),
        ));
    }

    /// What a visit ticks to come in (`docs/history/SIMPLIFY.md` §4.6): every file
    /// on the stick shown that Faraday reads, but a seed's by its name
    /// (which comes in through its own tick), one over 18 MiB, and one
    /// From the stick already by name and size.
    pub(crate) fn visit_default_in(&self) -> std::collections::BTreeSet<String> {
        let Some(stick) = self.stick.get(self.visit.stick) else {
            return Default::default();
        };
        stick
            .files
            .iter()
            .filter(|(n, size)| {
                stick.kind_of(n).is_some()
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
            .filter(|i| !self.in_the_backup(&i.name))
            .map(|i| i.name.clone())
            .collect()
    }

    /// Costs a hash of a whole file (through `hash`).
    /// [`Faraday::visit_all_out`], but with more than one vault file
    /// waiting, one vault for this stick (`docs/history/NEW-WALLET.md` §5.3): the
    /// first the last write did not write. The rest wait for the next
    /// stick.
    fn visit_default_out(&self, c: &Compute) -> std::collections::BTreeSet<String> {
        let mut out = self.visit_all_out();
        let vaults: Vec<&Item> = self
            .outbox
            .iter()
            .filter(|i| i.kind == FileKind::Vault)
            .collect();
        if vaults.len() > 1 {
            let written = |i: &Item| {
                self.visit
                    .receipt()
                    .as_ref()
                    .is_some_and(|r| r.holds(&i.name, &i.hash(c)))
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

    /// Lock asked for: by the person (`for_flow` false), or by a step of
    /// a backup flow. Asks first when a vault is open with something
    /// not in it, a seal waits, or the lock would end Make a new wallet
    /// (`docs/history/NEW-WALLET.md` §18); else locks.
    pub(crate) fn lock_ask(&mut self, for_flow: bool) {
        let c = self.compute();
        self.idle.set_lock_for_flow(for_flow);
        let (keys, wallets) = self.unsaved_now(&c);
        let sealed = self.vaults.open.iter().any(|v| v.changes > 0);
        if self.lock_ends_create(&c)
            || self.lock_would_lose()
            || !self.vaults.open.is_empty() && (sealed || !keys.is_empty() || !wallets.is_empty())
        {
            self.sheet = Some(Sheet::LockAsk);
        } else {
            self.lock();
        }
    }

    /// Costs what `lock_held` costs. A lock now would end Make a new
    /// wallet: its keys are kept nowhere yet and the lock is the person's
    /// own, not a flow's step.
    pub(crate) fn lock_ends_create(&self, c: &Compute) -> bool {
        self.lock_held(c) && !self.idle.lock_for_flow()
    }

    /// Lock, unless it would lose an open vault's changes: then nothing
    /// is sealed or wiped, the vaults stay open with their changes, and
    /// the Lock sheet says what a lock without sealing loses.
    fn lock(&mut self) {
        if self.lock_would_lose() {
            if self.seal_waits() {
                self.out.commands.push_back(Command::RequestEntropy);
            }
            self.sheet = Some(Sheet::LockAsk);
            return;
        }
        self.lock_now();
    }

    /// The state could not be kept. A lock that would lose the keys that
    /// wait for their backup does not go ahead.
    fn not_kept(&mut self) {
        if self.restart && self.flows.iter().any(|f| f.waiting()) {
            self.restart = false;
            self.next_mode.clear();
            self.toast(EN.toast_lock_not_kept);
        }
    }

    /// Lock: what can be sealed is, and the session goes.
    fn lock_now(&mut self) {
        // The code on show goes first, its text wiped: it may be an
        // entry's TOTP secret (`docs/QR.md` §7.1).
        self.qr.close();
        let c = self.compute();
        // A lock before Make a new wallet's keys are kept anywhere ends
        // it, as its Cancel does: nothing is left to come back to
        // (`docs/history/NEW-WALLET.md` §18).
        if self.lock_ends_create(&c) {
            self.create_drop();
        }
        self.flow_before_lock();
        self.vault_seal_all();
        // What the boot stick brought and was not imported is not kept:
        // its bytes are wiped as it drops.
        self.import.clear_all();
        // Nothing secret copied in outlives the process that read it
        // (`docs/history/PLAN.md` §5.3): the next process is clean, and these would be
        // in its memory and in `/run/faraday`. They are still on the stick
        // they came from.
        // A picture goes as what its codes hold, and as Other when none read.
        self.inbox.retain(|i| !i.dropped_at_lock());
        self.session.wipe();
        self.own_wipe();
        self.wallets.clear_spend();
        self.backup.close();
        self.message.clear();
        self.create.clear();
        self.restore.clear();
        self.keygen.forget();
        self.wordlist.clear();
        self.bip85.clear();
        self.explore.clear();
        self.secrets.clear();
        self.entry.reset();
        self.save_boxes_for_lock();
        self.restart = true;
        self.out.commands.push_back(Command::Exit);
    }

    /// Finish makes both results For the stick the moment the transaction is
    /// finished: the signed PSBT and the finished transaction. One the person
    /// removed is not made again unless they ask; neither is made where a
    /// version 2 PSBT cannot take the signatures back (`Spend::out_bytes`).
    fn spend_files_out(&mut self) {
        let spend = self.wallets.spend_mut();
        let Some(s) = spend.filter(|s| s.spend.finished.is_some()) else {
            return;
        };
        let stem = result_stem(&s.spend.source);
        let mut made: Vec<(String, Vec<u8>)> = Vec::new();
        if !s.out_signed {
            let Ok(bytes) = s.spend.out_bytes() else {
                return self.toast(EN.spend_v2_not_written);
            };
            s.out_signed = true;
            made.push((format!("{stem}-signed.psbt"), bytes));
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
    /// signers while more are needed.
    fn part_out(&mut self) {
        let Some(s) = self.wallets.spend().filter(|s| !s.complete) else {
            return;
        };
        let name = format!("{}-part.psbt", result_stem(&s.spend.source));
        let Ok(bytes) = s.spend.out_bytes() else {
            return self.toast(EN.spend_v2_not_written);
        };
        self.put_outbox(&name, bytes);
    }

    /// Costs a hash of a whole file (through `vault_changed`). The session
    /// strip's stage, computed, never stored (`docs/FLOWS.md`, Sidebar).
    pub fn session_stage(&self, c: &Compute) -> &'static str {
        if !self.stick.is_empty() || self.import.state().is_some() {
            EN.session_stage_bring
        } else if !self.outbox.is_empty() || self.vault_changed(c).is_some() {
            EN.session_stage_write
        } else if self.holds_secret() || !self.session.wallets.is_empty() {
            EN.session_stage_work
        } else if !self.inbox.is_empty() {
            EN.session_stage_open
        } else {
            EN.session_stage_bring
        }
    }

    /// The Inbox PSBT to offer first: the one with the fewest signatures,
    /// so a cosigner's signed copy never stands in for the transaction.
    /// Reads what each file said when it came in ([`Item::psbt`]).
    pub fn lead_psbt(&self) -> Option<usize> {
        self.inbox
            .iter()
            .enumerate()
            .filter_map(|(i, it)| Some((it.psbt()?.sigs, i)))
            .min()
            .map(|(_, i)| i)
    }

    /// Signatures the open spend's wallet needs.
    pub fn spend_needed(&self) -> usize {
        self.wallets.spend().map(|s| s.needed).unwrap_or(1)
    }

    fn act(&mut self, action: Action) {
        let asked = self.session.key_path_asked().is_some();
        let account_asked = self.session.account_asked().is_some();
        // A choice in a file's ⋮ menu closes the menu first.
        self.file_menu_chosen(action);
        self.act_moving_on(action);
        // A wallet's flow follows the plan and the keys the press left.
        self.wallet_flow_sync();
        // A board left takes its empty sticks with it.
        self.board_tidy();
        // The order the flows were begun in, and where each was left.
        self.flows_note();
        self.vault_shown_tidy();
        self.sweep_tidy();
        self.read_carried_tidy();
        self.key_path_after(asked, action);
        self.account_after(account_asked, action);
        self.osk_after(action);
    }

    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles lock, power and idle-timer actions (`Lock*`, `NotNow`, `Power*`, `Restart`, `Hold*`, `Idle*`).
    pub(crate) fn power_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::Lock => self.lock(),
            Action::LockUnsealed => self.lock_now(),
            Action::LockAsk => self.lock_ask(false),
            Action::NotNow => {
                self.next_mode.clear();
                self.sheet = None;
                self.stick.set_not_now(true);
            }
            Action::PowerAsk => {
                // An open vault's changes are never sealed at power-off:
                // the sheet says they are lost.
                let changed = self.vaults.open.iter().any(|v| v.changes > 0);
                if self.outbox.is_empty()
                    && !self.secure_boot_reminder()
                    && !changed
                    && !self.offers_restart()
                    && !self.lock_held(c)
                {
                    self.out.commands.push_back(Command::Exit);
                } else {
                    self.sheet = Some(Sheet::Power);
                }
            }
            Action::KeyPathHold => self.key_path_act(action),
            Action::PowerOff | Action::HoldPowerOff => {
                self.sheet = None;
                self.out.commands.push_back(Command::Exit);
            }
            Action::Restart | Action::HoldRestart => {
                self.sheet = None;
                self.restart_computer = self.offers_restart();
                self.out.commands.push_back(Command::Exit);
            }
            Action::IdleLock(m) => {
                self.settings.set_idle_lock_min(m);
                self.save_boxes();
            }
            Action::IdleOff(m) => {
                self.settings.set_idle_off_min(m);
                self.save_boxes();
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }

    /// Handles `Action::Cancel`, Back and Escape on a sheet: the sheet closes,
    /// by the way out its own kind has, and what it held is let go.
    pub(crate) fn cancel_act(&mut self, action: Action) {
        match action {
            // The ⋮ menu's way out is its own: back to the Add sheet it
            // came from (`docs/BACKUP-FLOW.md` §17.20).
            Action::Cancel if self.sheet == Some(Sheet::BoardMenu) => {
                self.board_act(board::BoardPress::MenuClose);
            }
            Action::Cancel
                if matches!(
                    self.sheet,
                    Some(Sheet::FileMenu | Sheet::Rename | Sheet::Details)
                ) =>
            {
                self.file_act(file_menu::FilePress::Close);
            }
            // Escape on the Load sheet is its Not now (F62).
            Action::Cancel if self.sheet == Some(Sheet::UnlockLoad) => {
                self.load_sheet_act(vault::VaultAction::LoadNotNow);
            }
            // Sign with {key}'s Cancel: the sheet closes, the files stay
            // ticked, nothing is signed (F63).
            Action::Cancel if self.sheet == Some(Sheet::GpgSign) => self.sheet = None,
            Action::Cancel if self.sheet == Some(Sheet::PassphraseSeeds) => self.pass_seeds_close(),
            Action::Cancel if self.sweep_scanning() => self.sweep_back(),
            Action::Cancel => {
                *self.visit.pull_mut() = None;
                self.vaults.clear_stick_save();
                self.devices.cancel();
                // A seed read and not loaded is wiped.
                self.seed_read.clear();
                self.stick.clear_delete_ask();
                self.visit.taken.clear();
                self.next_mode.clear();
                self.secret_cancel();
                self.wordlist.clear();
                self.inbox.clear_potential();
                // A transfer in parts survives closing the camera: the
                // parts read so far wait for the next scan.
                self.camera.keep_parts();
                if let Some(scan) = self.camera.take() {
                    self.out.commands.push_back(Command::CameraOff);
                    // A copy's scan closed by the person: one scan without
                    // a match (a match closes the camera itself).
                    if let ScanPurpose::CheckCopy(fp) = scan.purpose {
                        self.backup.copy_missed(fp.0);
                    }
                }
                self.learn_cancel();
                self.qr.close();
                self.glance.set_chart(None);
                self.glance.work_mut().secret.clear();
                self.glance.work_mut().ask = None;
                self.backup.close_pick();
                // A device still waiting comes back up.
                self.input_sheet();
            }
            // `act_inner` passes only `Action::Cancel`.
            _ => unreachable!(),
        }
    }

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn act_inner(&mut self, action: Action) {
        let c = self.compute();
        // A build whose vectors do not reproduce does nothing but exit.
        if self.selftest_failed() {
            if action == Action::PowerOff {
                self.out.commands.push_back(Command::Exit);
            }
            return;
        }
        // In the demo an action that would make a secret does nothing:
        // no sheet, no lead, no toast (`docs/DEMO.md` §2, §7).
        if self.demo_refuses(action) {
            return;
        }
        if self.stick_bar_press(action) {
            return;
        }
        // A chip held on the backup board is let go by a press anywhere
        // else (`docs/BACKUP-FLOW.md` §17.2).
        if !matches!(action, Action::Board(_)) {
            self.board_let_go();
        }
        // A chart's sheet leads into a flow: whatever its rows press, the
        // sheet goes first.
        if self.sheet == Some(Sheet::Chart) && !matches!(action, Action::Chart(_)) {
            self.sheet = None;
            self.glance.set_chart(None);
        }
        // While something that can capture the screen or the keys is
        // attached and not let through, what shows a secret waits under
        // its sheet (`docs/STARTER.md` §3.1a).
        if self.device_holds(action) {
            // Reminders alone ride on the sheet that asks for the stick
            // to be pulled: one sheet, and pulling it is going on.
            let pull = self.selftest_passed()
                && !self.stick.is_empty()
                && self.pull_what(action).is_some();
            if pull && self.devices.held.is_empty() {
                *self.visit.pull_mut() = Some(action);
                self.sheet = Some(Sheet::Pull);
                return;
            }
            self.device_hold(action);
            return;
        }
        // Keys load only with no stick attached: what loads one waits,
        // under a sheet, for the stick to be pulled.
        if self.selftest_passed() && !self.stick.is_empty() && self.pull_what(action).is_some() {
            *self.visit.pull_mut() = Some(action);
            self.sheet = Some(Sheet::Pull);
            return;
        }
        match action {
            Action::SelfTestRun => {
                self.selftest = Some(faraday_selftest::run(&c));
                if self.selftest_failed() {
                    self.sheet = None;
                }
            }
            Action::Flow(a) => self.flow_act(a),
            Action::Vault(v) => {
                self.vault_act(v);
                // What changed in an open vault is what a lock remembers.
                self.vault_summaries_refresh();
            }
            Action::Import(a) => self.import_act(a),
            Action::Host(_) => {}
            Action::Family(f) => self.family_act(f),
            Action::Vanity(v) => self.vanity_act(v),
            Action::Learn | Action::LearnClose | Action::How => self.learn_press(action),
            Action::NavMenu(to) => self.nav_menu(to),
            Action::LearnShares => self.learn_shares(),
            Action::LearnTopic(t) => self.learn_topic(t),
            Action::LearnSpend | Action::LearnPin => self.learn_button(action),
            Action::Osk(p) => self.osk_press(p),
            a @ (Action::ScanPaste
            | Action::PasteWords
            | Action::ScanCamera(..)
            | Action::ScanPart
            | Action::ScanSeed
            | Action::ScanKey(..)
            | Action::Scan
            | Action::ScanFor(_)
            | Action::ReadGo(_)
            | Action::ReadAgain
            | Action::ReadShow) => self.scan_act(a),
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
            | Action::PRecord(_)
            | Action::POut => self.bip85_act(action),
            Action::Silent
            | Action::SStep(_)
            | Action::SKey(_)
            | Action::SLabel(_)
            | Action::SQr(_)
            | Action::SRecord
            | Action::SScanVault
            | Action::SScanOut
            | Action::SDescriptorOut(_)
            | Action::SNext
            | Action::SAddWallet
            | Action::SWallet(_)
            | Action::SCheck(_) => self.silent_act(action),
            Action::Explore
            | Action::XKey(_)
            | Action::XPreset(_)
            | Action::XScript(_)
            | Action::XIndex(_)
            | Action::X(_) => self.explore_act(action),
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
            a @ (Action::SecretVault
            | Action::SecretHold
            | Action::SecretUnprotected
            | Action::SecretForm(..)) => self.secret_act(a),
            Action::LearnPage(i) => self.learn.show_page(i),
            Action::LearnStep(n) => self.learn.tick_step(n),
            Action::WordList(a) => self.wordlist.act(a, &mut self.sheet),
            Action::KeyGen(_)
            | Action::KStep(_)
            | Action::KStart(_)
            | Action::KOthers
            | Action::KWords(_)
            | Action::KOtherLengths
            | Action::KWay(_)
            | Action::KGroup(_)
            | Action::KMix(_)
            | Action::KRoll(_)
            | Action::KRoller(_)
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
            | Action::KSlipWords(_)
            | Action::KSlipM(_)
            | Action::KSlipN(_)
            | Action::KShare(_)
            | Action::KeyGenSlip39 => self.keygen_act(action),
            Action::Codex32(p) => self.keygen_codex32(p),
            Action::Nav(s @ (Screen::Transfer | Screen::Randomness)) => self.nav_open(s),
            Action::Back => self.back(),
            Action::Own(a) => self.own_press(a),
            Action::Resume(g) => self.flow_resume(g),
            Action::CancelFlow => self.flow_cancel(),
            Action::CancelFlowNow => self.flow_cancel_now(),
            a @ (Action::TransferSend(..)
            | Action::TransferReceive
            | Action::TransferOpenFolder) => self.transfer_act(a),
            Action::SeedRead(c) => self.seed_read_act(c),
            Action::Sweep(a) => self.sweep_act(a),
            a @ (Action::DeleteFiles
            | Action::DeleteWhat(..)
            | Action::DeletePick(..)
            | Action::DeletePasses(..)
            | Action::DeleteGo
            | Action::ZeroStop) => self.wipe_act(a),
            Action::OpenInstitute => {
                if self.online {
                    self.out
                        .storage_out
                        .push_back(StorageCommand::OpenInstitute);
                }
            }
            a @ (Action::BackupsOf(..) | Action::BackupsPage(..)) => self.backups_act(a, &c),
            Action::Nav(s) => {
                self.glance.set_focus(None);
                self.panes_nav(s);
                // Spend's way back is to where it was opened from.
                if s == Screen::Family && self.screen != Screen::Family {
                    self.family.opened_from(self.screen);
                }
                self.screen = s;
                self.osk_leave();
                self.list_offset = 0.0;
                self.wallets.set_card_offset(0.0);
                *self.visit.files_out_offset_mut() = 0.0;
                self.glance.set_open(false);
                self.wallets.set_renaming(None);
                if s == Screen::Visit {
                    self.visit.out = self.visit_default_out(&c);
                    self.visit.inn = self.visit_default_in();
                    self.visit.from_inbox.clear();
                    self.visit.out_offset = 0.0;
                }
            }
            a @ (Action::Entry(..)
            | Action::EntryWordCount(..)
            | Action::EntryClear
            | Action::EntryAdd
            | Action::EntryLanguages
            | Action::EntryLanguage(..)
            | Action::EntryKey(..)
            | Action::EntryKeyBack
            | Action::EntryShift
            | Action::EntryCandidate(..)
            | Action::EntryForm(..)
            | Action::EntryOtherForms
            | Action::EntryPart
            | Action::EntryRecover
            | Action::EntryPassphrase
            | Action::EntryField
            | Action::EntryWallet(..)
            | Action::Codex32Correct(..)
            | Action::Codex32Form(..)) => self.entry_act(a),
            a @ (Action::PickWallet(..)
            | Action::PickKey(..)
            | Action::RemoveWallet(..)
            | Action::RemoveKeys
            | Action::StartSpend(..)
            | Action::Step(..)
            | Action::StepNext(..)
            | Action::ReviewHold
            | Action::ToggleTable
            | Action::SignHere
            | Action::Collect(..)
            | Action::SignedToOutbox
            | Action::TxToOutbox
            | Action::Primary
            | Action::LoadWallet(..)
            | Action::LoadKey(..)
            | Action::PublicOut(..)
            | Action::PublicMore(..)
            | Action::PublicRemove(..)
            | Action::WalletKeyOut(..)
            | Action::WalletKeyBsms(..)
            | Action::OpenWallet(..)
            | Action::WriteAsk
            | Action::Rename) => self.wallet_act(a),
            a @ (Action::InboxRemove(..)
            | Action::OutboxRemove(..)
            | Action::InboxLoad
            | Action::InboxChoose(..)
            | Action::PotentialOpen(..)
            | Action::XpubOpen(..)
            | Action::PotentialKind(..)
            | Action::PotentialShow
            | Action::PotentialType
            | Action::PotentialMake) => self.inbox_act(a, &c),
            a @ (Action::VisitStick(..)
            | Action::VisitSettings
            | Action::VisitOut(..)
            | Action::VisitIn(..)
            | Action::VisitInAll
            | Action::VisitOutAll
            | Action::VisitInbox(..)
            | Action::VisitBar(..)
            | Action::VisitWrite
            | Action::VisitReplace
            | Action::VisitKeepBoth
            | Action::VisitCopy
            | Action::VisitCopyAndLoad
            | Action::VisitFrom(..)) => self.visit_act(a),
            a @ (Action::Lock
            | Action::LockUnsealed
            | Action::LockAsk
            | Action::NotNow
            | Action::PowerAsk
            | Action::PowerOff
            | Action::Restart
            | Action::HoldPowerOff
            | Action::HoldRestart
            | Action::IdleLock(..)
            | Action::IdleOff(..)) => self.power_act(a, &c),
            a @ (Action::KeyPathHold | Action::KeyPathCancel) => self.key_path_act(a),
            Action::LoadAccount(a) => self.account_act(a),
            a @ (Action::RegisterOpen
            | Action::RegisterField
            | Action::RegisterHold
            | Action::RegisterCancel
            | Action::RegisterReading(_)) => self.register_act(a),
            a @ (Action::InputUse(..)
            | Action::InputIgnore(..)
            | Action::InputTarget(..)
            | Action::Mode(..)
            | Action::ModeSheet
            | Action::DevicesCheck
            | Action::DevicesContinue
            | Action::DevicesHold
            | Action::DevicesUnderstood) => self.devices_act(a),
            Action::PassChip(f) => self.demo_pass_chip(f),
            a @ (Action::Samples
            | Action::SampleTick(..)
            | Action::SamplesAll
            | Action::SamplesLoad
            | Action::UseSample
            | Action::ScanSample
            | Action::SamplePsbt) => self.samples_act(a),
            a @ (Action::RFromVault(..)
            | Action::RestoreWallet
            | Action::RStep(..)
            | Action::RVaults(..)
            | Action::RVaultN(..)
            | Action::RNext(..)
            | Action::RKind(..)
            | Action::RMoreKinds
            | Action::RFromFiles
            | Action::RAbsent(..)
            | Action::RSlotWords
            | Action::RSlotScan
            | Action::RUse(..)
            | Action::RShare(..)
            | Action::RRebuild
            | Action::RSeeds
            | Action::KeyWallet(..)
            | Action::RestoreShares) => self.restore_act(a, &c),
            a @ Action::Cancel => self.cancel_act(a),
            Action::File(p) => self.file_act(p),
            Action::About(k) => self.panel.toggle_about(self.screen, k),
            Action::Guided(on) => {
                if on != self.guided() {
                    // The switch's pill slides from where it shows now.
                    let from = self.guided_shown();
                    self.guided_moving = (!self.reduce_motion()).then_some((from, None));
                }
                self.settings.set_guided(on);
                self.save_boxes();
            }
            Action::SealAmounts(on) => {
                self.memory.set_seals(on);
                self.save_boxes();
            }
            a @ (Action::TzStep(..)
            | Action::OpenSetTime
            | Action::SetTimeFocus(..)
            | Action::SetTimeSet
            | Action::SetTimeKeep
            | Action::SetTimeBack) => self.time_act(a),
            a @ (Action::OpenCodes | Action::CodesOpen(..) | Action::CodesAdd) => self.codes_act(a),
            Action::Chart(c) => self.chart_act(c),
            Action::Glance(open) => {
                self.glance.set_open(open && self.screen == Screen::Wallets);
                self.list_offset = 0.0;
            }
            a @ (Action::BackupChecklist(..)
            | Action::Backup(..)
            | Action::BQ(..)
            | Action::BQNext(..)
            | Action::BAnswer(..)
            | Action::BName(..)
            | Action::BChecklist
            | Action::BPlan
            | Action::BWallets
            | Action::BStep(..)
            | Action::BNext(..)
            | Action::BKey(..)
            | Action::BReveal
            | Action::BXor(..)
            | Action::BPaperHide
            | Action::BCompact(..)
            | Action::BPin(..)
            | Action::BRows(..)
            | Action::BCheck
            | Action::BScan
            | Action::BCheckClear
            | Action::BKeepWords
            | Action::BVaultSave(..)
            | Action::BVaultLoad(..)
            | Action::BFilesMake
            | Action::BackupFirst(..)
            | Action::BNewVault(..)
            | Action::BFile
            | Action::BWords(..)
            | Action::BOmit(..)
            | Action::BOut(..)
            | Action::BC32(..)) => self.backup_act(a, &c),
            Action::Board(p) => self.board_act(p),
            Action::Catalog(i) => self.catalog_go(usize::from(i)),
            a @ (Action::DecodeFinished
            | Action::DecodeInbox(..)
            | Action::QrDecoded
            | Action::DecodeHex
            | Action::SpendHex) => self.decode_act(a, &c),
            Action::NonceCheck(p) => self.nonce_check_act(p, &c),
            a @ (Action::SignMessage
            | Action::MStep(..)
            | Action::MNext(..)
            | Action::MWallet(..)
            | Action::MIndex(..)
            | Action::MType
            | Action::MFormat(..)
            | Action::MSign
            | Action::MOut
            | Action::MQr
            | Action::CheckMessage(..)) => self.message_act(a, &c),
            a @ (Action::CreateWallet
            | Action::CreateOver
            | Action::CStep(..)
            | Action::CNext(..)
            | Action::CKind(..)
            | Action::CBackup
            | Action::CVaults(..)
            | Action::CVaultN(..)
            | Action::PlanAddPass
            | Action::CMoreKinds
            | Action::CKindAbout
            | Action::CM(..)
            | Action::CN(..)
            | Action::CSlotVault(..)
            | Action::CAccount(..)
            | Action::CPathChange
            | Action::NewWalletFrom(..)
            | Action::CSlotHere(..)
            | Action::CSlotFile(..)
            | Action::CSlotLater(..)
            | Action::CKeyOut(..)
            | Action::CKeyQr(..)
            | Action::CKeyBsms(..)
            | Action::CSlotClear(..)
            | Action::CPassOpen(..)
            | Action::CPassField(..)
            | Action::CPassLock) => self.create_act(a, &c),
            Action::Seeds(a) => self.seeds_act(a),
            Action::Slide(id, v) => self.slide(id, v),
            a @ (Action::PdfInbox(..)
            | Action::PdfOutbox(..)
            | Action::QrSigned
            | Action::QrPart
            | Action::QrWallet(..)
            | Action::ShowCode(..)
            | Action::CodePng(..)
            | Action::QrPng
            | Action::QrOutbox(..)
            | Action::QrFormat(..)
            | Action::QrPartSize(..)
            | Action::QrSpeed(..)) => self.qr_act(a, &c),
            a @ (Action::Scale(_)
            | Action::Rotation(_)
            | Action::TextSize(_)
            | Action::ReduceMotion(_)
            | Action::Theme(_)) => self.appearance_act(a),
            Action::SettingsSection(k) => self.settings_section(k),
            // Acted on before (`act_inner`), or never pressed (a mark).
            Action::StickPill(_) | Action::Mark(..) => {}
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
        let fps: Vec<_> = w
            .policy()
            .keys()
            .iter()
            .filter_map(|k| k.fingerprint())
            .collect();
        self.session
            .keys
            .all()
            .enumerate()
            // A key with words, or one whose seed is a codex32 set it
            // knows (`docs/CODEX32.md` §9.4): its backup places that set.
            .filter(|(_, k)| (k.has_words() || k.codex32.is_some()) && fps.contains(&k.fingerprint))
            .map(|(i, _)| i)
            .collect()
    }

    /// Wallet `w`'s seeds, in its order, as the plan and the panel list
    /// them: a name each (its fingerprint, or a share not here by number)
    /// and the session key that holds it, when its words are here.
    ///
    /// Works out [`crate::drawing::View::backup_seed_list`] from the state as it is.
    pub fn backup_seed_list_now(&self, w: usize) -> Vec<(String, Option<usize>)> {
        match self.session.wallets.get(w) {
            Some(wallet) => self.backup_seed_list_of(wallet),
            None => Vec::new(),
        }
    }

    /// [`Faraday::backup_seed_list_now`] for a wallet that need not be loaded.
    pub(crate) fn backup_seed_list_of(
        &self,
        wallet: &wallet::Wallet,
    ) -> Vec<(String, Option<usize>)> {
        let here = self.backup_keys_of(wallet);
        let mut fps: Vec<faraday_bip::keys::Fingerprint> = Vec::new();
        for f in wallet
            .policy()
            .keys()
            .iter()
            .filter_map(|k| k.fingerprint())
        {
            if !fps.contains(&f) {
                fps.push(f);
            }
        }
        fps.into_iter()
            .map(|fp| {
                let key = here
                    .iter()
                    .copied()
                    .find(|&k| self.session.keys[k].fingerprint == fp);
                (fp_text(fp), key)
            })
            .collect()
    }

    /// Costs a map of the plan and its check the first time.
    /// [`crate::drawing::Kept::plan_mapped`] on the state's own kept lists.
    pub(crate) fn plan_mapped(
        &self,
        c: &Compute,
        shape: &plan::Shape,
        a: &plan::Answers,
    ) -> std::rc::Rc<plan::Mapped> {
        self.kept.plan_mapped(c, shape, a)
    }

    /// What wallet `w`'s plan is for: its quorum, its seeds and whether
    /// its description splits. Searches the session's keys; kept for the frame while one is drawn.
    ///
    /// Works out [`crate::drawing::View::plan_shape`] from the state as it is.
    pub fn plan_shape_now(&self, w: usize) -> plan::Shape {
        let Some(wallet) = self.session.wallets.get(w) else {
            return plan::Shape {
                m: 1,
                keys: 1,
                seeds: Vec::new(),
                splits: false,
                vaults: 0,
            };
        };
        self.plan_shape_of(wallet)
    }

    /// [`Faraday::plan_shape_now`] for a wallet that need not be loaded.
    pub(crate) fn plan_shape_of(&self, wallet: &wallet::Wallet) -> plan::Shape {
        let (m, n) = Session::quorum(wallet);
        let seeds: Vec<plan::Seed> = self
            .backup_seed_list_of(wallet)
            .into_iter()
            .map(|(name, key)| plan::Seed {
                name,
                here: key.is_some(),
                passphrase: key.is_some_and(|k| self.session.keys[k].has_passphrase),
            })
            .collect();
        // The vaults set up before the keys are the plan's first vaults.
        let vaults = self
            .wallet_flow_active(wallet.checksum())
            .and_then(|i| self.flows[i].wallet.as_ref())
            .map_or(0, |w| w.shape.vaults);
        plan::Shape {
            m: m.clamp(1, seeds.len().max(1)),
            keys: n.max(1),
            seeds,
            splits: wallet_backup::splits(wallet),
            vaults,
        }
    }

    /// [`Faraday::backup_public`] for wallet `w` under answers `a`.
    pub(crate) fn backup_public_for(w: &wallet::Wallet, a: &plan::Answers) -> Vec<u8> {
        use faraday_bip::policy::{Template, Wrapper};
        use plan::software as sw;
        let config = wallet_backup::multisig_config(w, None).is_some();
        let bsms = matches!(
            w.policy().template(),
            Template::Multi {
                wrapper: Wrapper::Wsh | Wrapper::ShWsh,
                ..
            }
        );
        let core = w.policy().silent().is_none();
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

    /// Costs what `account_key_names` costs.
    /// [`Faraday::backup_public_names`] for loaded wallet `wi` under
    /// answers `a`.
    ///
    /// Works out [`crate::drawing::View::backup_public_names_for`] from the state as it
    /// is.
    pub(crate) fn backup_public_names_for_now(
        &self,
        c: &Compute,
        wi: usize,
        a: &plan::Answers,
    ) -> Vec<String> {
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
                1 => picture_names(
                    &format!("{stem}-descriptor"),
                    "-",
                    &w.policy().to_descriptor_checksummed(),
                )
                .into_iter()
                .next(),
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
        // Each key loaded here, its account key and, on a `wsh` or
        // `sh(wsh)` multisig at Create's own account, its BIP 129 record.
        out.extend(self.account_key_names(c, wi));
        out
    }

    /// Costs what `wallet_key` costs. The names of the account key files of
    /// wallet `wi`, without making them: the checklist asks for them on
    /// every frame.
    pub(crate) fn account_key_names(&self, c: &Compute, wi: usize) -> Vec<String> {
        let mut out = Vec::new();
        for slot in self.wallet_keys_here(wi) {
            if let Ok(k) = self.wallet_key(c, wi, slot) {
                out.push(format!("xpub-{}.txt", k.fp));
                // The BIP 129 record is signed by the key, for a `wsh` or
                // `sh(wsh)` multisig at Create's own account.
                if matches!(
                    k.standard,
                    Some(create::NewKind::Multi | create::NewKind::MultiNested)
                ) {
                    out.push(format!("xpub-{}-bsms.txt", k.fp));
                }
            }
        }
        out
    }

    /// Costs what `wallet_key` and `wallet_key_record` cost. The account
    /// key files of the keys of wallet `wi` held here: the key as
    /// `[fingerprint/path]xpub`, and its signed BIP 129 record where the
    /// kind has one. A cosigner's key not loaded here has neither; its
    /// account key is in the descriptor.
    pub(crate) fn account_key_files(&self, c: &Compute, wi: usize) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        for slot in self.wallet_keys_here(wi) {
            if let Ok(k) = self.wallet_key(c, wi, slot) {
                out.push((
                    format!("xpub-{}.txt", k.fp),
                    format!("# Account xpub {}, {}\n{}\n", k.fp, k.kind, k.text).into_bytes(),
                ));
            }
            if let Ok((fp, text)) = self.wallet_key_record(c, wi, slot) {
                out.push((format!("xpub-{fp}-bsms.txt"), text.into_bytes()));
            }
        }
        out
    }

    /// Whether a file of key `k`'s seed, for the wallet whose files are
    /// named `stem`, is For the stick or was written by the last visit.
    pub(crate) fn seed_file_made(&self, stem: &str, k: &faraday_keys::KeyInfo) -> bool {
        let fps = fp_text(k.fingerprint).to_lowercase();
        ["words.txt", "seedqr.png", "compactseedqr.png"]
            .iter()
            .any(|end| {
                let name = format!("{stem}-{fps}-{end}");
                self.outbox.iter().any(|i| i.secret && i.name == name)
                    || self
                        .visit
                        .receipt()
                        .as_ref()
                        .is_some_and(|r| r.wrote(&name))
                    || self.wallet_wrote(&name)
            })
    }

    /// Costs what `vault_holds_seed` costs. Whether open vault `o` holds a
    /// seed the plan puts into another vault and not vault `v`: it is not
    /// offered for vault `v`.
    pub fn backup_vault_taken(&self, c: &Compute, v: usize, o: usize) -> bool {
        let (_, others) = self.backup_vault_seeds(v);
        others
            .iter()
            .any(|&k| self.vault_holds_seed(c, o, k, false))
    }

    /// Costs what `backup_vault_wanted` and `backup_open_fits` cost. The
    /// open vault that does the plan's vault `v`: it holds every seed the
    /// plan puts into `v`, the wallet where the plan puts it there, and no
    /// seed only another vault is to hold. A vault set up before the keys
    /// is the plan's vault of its number; no other plan vault takes it. Its
    /// place among the open vaults.
    pub(crate) fn backup_vault_open(&self, c: &Compute, v: usize) -> Option<usize> {
        self.backup_vault_wanted(c, v)?;
        if let Some(o) = self.backup_vault_set_up(v) {
            return self.backup_open_fits(c, v, o).then_some(o);
        }
        let theirs = self.backup_vaults_set_up_but(v);
        (0..self.vaults.open.len())
            .find(|&o| !theirs.contains(&o) && self.backup_open_fits(c, v, o))
    }

    /// What the plan's vault `v` is called where its seeds are chosen:
    /// "Vault 1", and the name of the vault set up for it before the keys.
    pub(crate) fn backup_vault_title(&self, v: usize) -> String {
        let n = (v + 1).to_string();
        match self
            .backup_vault_set_up(v)
            .map(|o| &self.vaults.open[o].name)
        {
            Some(name) if !name.is_empty() => fill(EN.plan_vault_names, &[&n, name]),
            _ => fill(EN.plan_vault, &[&n]),
        }
    }

    /// The open vault that was set up before the keys for the plan's vault
    /// `v`, when the wallet's flow has one.
    pub(crate) fn backup_vault_set_up(&self, v: usize) -> Option<usize> {
        let i = self.wallet_flow_of_backup()?;
        self.flow_open_index(i, v)
    }

    /// The open vaults set up for plan vaults other than `v`.
    fn backup_vaults_set_up_but(&self, v: usize) -> Vec<usize> {
        let Some(i) = self.wallet_flow_of_backup() else {
            return Vec::new();
        };
        (0..self.flows[i].plan.vaults.len())
            .filter(|&w| w != v)
            .filter_map(|w| self.flow_open_index(i, w))
            .collect()
    }

    /// The open vault the checklist puts plan vault `v` in: the vault set
    /// up for it, else the current one if no other plan vault is its.
    pub(crate) fn backup_vault_target(&self, v: usize) -> Option<usize> {
        if let Some(o) = self.backup_vault_set_up(v) {
            return Some(o);
        }
        let cur = self.vaults.current;
        self.vaults.open.get(cur)?;
        (!self.backup_vaults_set_up_but(v).contains(&cur)).then_some(cur)
    }

    /// Whether open vault `o` holds session key `k`'s passphrase as a
    /// passphrase record: its fingerprint and its passphrase.
    pub(crate) fn vault_holds_passphrase(&self, o: usize, k: usize) -> bool {
        use faraday_vault::records::{field, kind};
        let (Some(open), Some(key)) = (self.vaults.open.get(o), self.session.keys.info(k)) else {
            return false;
        };
        let fp = key.fingerprint.0;
        self.vaults
            .records(open.id)
            .of(kind::PASSPHRASE)
            .any(|(_, r)| {
                r.public(field::PASSPHRASE_KEY) == Some(&fp[..])
                    && self
                        .vaults
                        .store()
                        .passphrase_record_is(r, &self.session.keys, k)
            })
    }

    /// Costs what `share_record_is` costs. Whether open vault `o` holds
    /// share `j` of the wallet with checksum `cs` (type 13, told by its
    /// field 5 and field 3), with this text.
    pub(crate) fn vault_holds_share(
        &self,
        c: &Compute,
        o: usize,
        cs: &str,
        j: usize,
        text: &str,
    ) -> bool {
        use faraday_vault::records::field;
        self.vaults.open.get(o).is_some_and(|x| {
            self.vaults.records(x.id).iter().any(|(_, r)| {
                share_record_is(c, r, cs, Some(j)) && r.public_text(field::SHARE) == Some(text)
            })
        })
    }

    /// The text a vault keeps a wallet under, as Vaults saves it.
    pub(crate) fn wallet_text(w: &wallet::Wallet) -> String {
        match w.policy().silent() {
            Some(s) => s.to_text(),
            None => w.policy().to_descriptor_checksummed(),
        }
    }

    /// Costs what `plan_load_of` costs. The plan an open vault keeps for
    /// wallet `w` (record type 11): its answers and the places' names. None
    /// when no open vault has one, or it is not for a wallet of this shape.
    fn plan_load(
        &self,
        c: &Compute,
        w: usize,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)> {
        self.plan_load_of(c, self.session.wallets.get(w)?, shape)
    }

    /// Costs what `plan_in_vault` costs. [`Faraday::plan_load`] for a
    /// wallet that need not be loaded.
    pub(crate) fn plan_load_of(
        &self,
        c: &Compute,
        wallet: &wallet::Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)> {
        (0..self.vaults.open.len()).find_map(|v| self.plan_in_vault(c, v, wallet, shape))
    }

    /// Costs what `described_of` costs. The plan open vault `v` keeps for
    /// `wallet` (its type 11 record), read against `shape`, with the
    /// places' names.
    pub(crate) fn plan_in_vault(
        &self,
        c: &Compute,
        v: usize,
        wallet: &wallet::Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)> {
        use faraday_vault::records::{field, kind};
        let want = wallet::same_wallet(wallet.policy());
        let o = self.vaults.open.get(v)?;
        self.vaults
            .described_of(c, o.id, kind::PLAN)
            .find_map(|(_, r, d)| {
                if d.same != want {
                    return None;
                }
                let a = plan::Answers::from_text(shape, r.public_text(field::PLAN_ANSWERS)?)?;
                let names = r
                    .public_all(field::PLAN_PLACE)
                    .map(|b| String::from_utf8_lossy(b).into_owned())
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
        let c = self.compute();
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return;
        };
        let v = self
            .plan_vault_of(&c, w)
            .filter(|_| self.vaults.open.get(self.vaults.current).is_some())
            .unwrap_or(self.vaults.current);
        let (wi, answers, names, extras) = (
            b.wallet,
            b.answers.clone(),
            b.names.clone(),
            b.extras.clone(),
        );
        self.plan_store(v, wi, &answers, &names, &extras, None);
        // And in each other open vault the wallet's flow keeps, so that
        // every vault holds the plan with its places' names
        // (`docs/BACKUP-FLOW.md` §15.7).
        if let Some(i) = self.wallet_flow_of_backup() {
            let others: Vec<usize> = (0..self.flows[i].plan.vaults.len())
                .filter_map(|pv| self.flow_open_index(i, pv))
                .filter(|&o| o != v)
                .collect();
            for o in others {
                self.plan_store(o, wi, &answers, &names, &extras, None);
            }
        }
    }

    /// Costs what `described_of` costs. The open vault that keeps a plan
    /// (record type 11) for `wallet`.
    pub(crate) fn plan_vault_of(&self, c: &Compute, wallet: &wallet::Wallet) -> Option<usize> {
        use faraday_vault::records::kind;
        let want = wallet::same_wallet(wallet.policy());
        (0..self.vaults.open.len()).find(|&v| {
            self.vaults
                .described_of(c, self.vaults.open[v].id, kind::PLAN)
                .any(|(_, _, d)| d.same == want)
        })
    }

    /// Writes loaded wallet `wi`'s plan into open vault `v` as record type
    /// 11, over the one it kept for the wallet: `answers`, the places'
    /// `names`, what each spot holds, work a later plan dropped
    /// (`extras`), and the notes on its map (§9.5): `notes`, or those the
    /// record kept.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn plan_store(
        &mut self,
        v: usize,
        wi: usize,
        answers: &plan::Answers,
        names: &[String],
        extras: &[String],
        notes: Option<&[plan::Note]>,
    ) {
        let c = self.compute();
        use faraday_vault::records::{Record, field, kind};
        let Some(w) = self.session.wallets.get(wi) else {
            return;
        };
        let Some(open) = self.vaults.open.get(v) else {
            return;
        };
        let shape = self.plan_shape_now(wi);
        // The keys made here with their backup pending, by fingerprint
        // (`docs/history/NEW-WALLET.md` §14.3).
        let mut text = answers.to_text();
        for fp in self.wallet_held_stored(&c, wi) {
            text.push_str(&format!("held {}\n", fp_text(fp)));
        }
        let ours: Vec<usize> = self
            .vaults
            .described_of(&c, open.id, kind::PLAN)
            .filter(|(_, _, d)| d.same == wallet::same_wallet(w.policy()))
            .map(|(i, _, _)| i)
            .collect();
        let text = plan::text::with_seals(&text, &self.vaults.plan_seals_kept(open.id, &ours));
        let mut record = Record::new(kind::PLAN)
            .with(field::PLAN_WALLET, Self::wallet_text(w).as_bytes())
            .with(field::PLAN_ANSWERS, text.as_bytes());
        for p in 0..answers.places {
            let name = names.get(p).map_or("", |n| n.trim());
            record.push(field::PLAN_PLACE, name.as_bytes());
        }
        // A place with what its sticks keep.
        for spot in plan::place_map(&c, &shape, answers) {
            let at = match spot.at {
                // The map's field names a place by number only; the name
                // itself lives in field 3 (`field::PLAN_PLACE`) alone.
                plan::At::Place(p) | plan::At::Stick(p, _) => {
                    fill(EN.plan_store_place, &[&(p + 1).to_string()])
                }
                plan::At::Vault(v) => answers.vault_name(&shape, v),
                plan::At::Files => fill(EN.plan_store_of_files, &[self.medium.cap()]),
                plan::At::Software => EN.plan_store_watch_only.to_string(),
                plan::At::Away => EN.plan_store_on_its_own.to_string(),
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
        let same = |i: usize| ours.contains(&i);
        // The notes on the map: those given, else those the record kept.
        let kept: Vec<String> = match notes {
            Some(n) => n.iter().map(plan::Note::to_text).collect(),
            None => self
                .vaults
                .records(open.id)
                .iter()
                .filter(|(i, _)| same(*i))
                .flat_map(|(_, r)| {
                    r.public_all(field::PLAN_NOTE)
                        .map(|b| String::from_utf8_lossy(b).into_owned())
                        .collect::<Vec<_>>()
                })
                .collect(),
        };
        for n in &kept {
            record.push(field::PLAN_NOTE, n.as_bytes());
        }
        // Kept as it was: nothing to write back.
        let unchanged = self
            .vaults
            .records(open.id)
            .iter()
            .any(|(i, r)| same(i) && r.same_fields(&record));
        if unchanged {
            return;
        }
        let name = open.name.clone();
        let id = open.id;
        let old: Vec<faraday_vault::records::Record> = self
            .vaults
            .records(id)
            .iter()
            .filter(|(i, _)| same(*i))
            .filter_map(|(_, r)| r.to_public())
            .collect();
        if !ours.is_empty() {
            self.vaults.edit(id, |s| {
                s.remove_where(id, |r| r.home().is_some_and(|(_, i)| same(i)))
            });
        }
        if !self.vault_push_in(
            v,
            record,
            &fill(EN.plan_store_the_plan, &[&name.to_string()]),
        ) {
            // It did not fit: the record it kept stays.
            self.vaults.edit(id, |s| {
                for r in old {
                    s.insert(id, r);
                }
            });
        } else if !old.is_empty() {
            self.vaults.changed(id);
        }
        self.vaults.seals_write(&c, v);
    }

    /// Costs a QR encoding (through `picture_codes`). The labelled pictures
    /// among a wallet's public files `what` (as [`Faraday::public_out`]
    /// numbers them): for 8, the descriptor's code, in parts past one code;
    /// for 4, each split share's code, one beside each share's sheet and
    /// text. None for any other file.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub fn public_pictures(&self, c: &Compute, wallet: usize, what: u8) -> Vec<picture::Labelled> {
        let Some(w) = self.public_wallet(c, wallet) else {
            return Vec::new();
        };
        let stem = file_stem(&w.name);
        let net = self.session.network();
        let shape = if net.is_mainnet() {
            Session::shape(w)
        } else {
            format!("{} · {}", Session::shape(w), net.name())
        };
        let keys = w.policy().keys();
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
            let codes = picture_codes(c, text).unwrap_or_default();
            let k = codes.len();
            let names = picture_names(name, part, text);
            codes
                .into_iter()
                .zip(names)
                .enumerate()
                .map(|(j, (code, name))| {
                    let mut label = if k == 1 {
                        Vec::new()
                    } else {
                        vec![fill(
                            EN.public_pictures_part,
                            &[&(j + 1).to_string(), &k.to_string()],
                        )]
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
                let text = w.policy().to_descriptor_checksummed();
                let all = fps(&|_| true);
                let mut lines = vec![shape];
                if !all.is_empty() {
                    let word = if all.len() == 1 {
                        EN.public_pictures_key
                    } else {
                        EN.public_pictures_keys
                    };
                    lines.push(format!("{word} {}", all.join(" · ")));
                }
                if let Some((_, sum)) = text.rsplit_once('#') {
                    lines.push(fill(EN.public_pictures_descriptor, &[sum]));
                }
                lines.push(EN.public_pictures_public_watch.to_string());
                pictures(&format!("{stem}-descriptor"), "-", &text, lines)
            }
            4 => {
                let (m, n) = Session::quorum(w);
                // As the shares' sheets and text are split.
                let omit = self
                    .backup
                    .state()
                    .map_or(m.saturating_sub(1), |b| b.answers.omit);
                let plan = wallet_backup::split_plan(n, m, omit);
                let mut out = Vec::new();
                for (i, row) in plan.iter().enumerate() {
                    let Some(text) = wallet_backup::split_share(w, row) else {
                        continue;
                    };
                    let mut lines = vec![
                        fill(
                            EN.public_pictures_share,
                            &[
                                &(i + 1).to_string(),
                                &plan.len().to_string(),
                                &m.to_string(),
                            ],
                        ),
                        shape.clone(),
                    ];
                    let held = fps(&|k| row.contains(&k));
                    if !held.is_empty() {
                        lines.push(fill(
                            EN.public_pictures_holds,
                            &[&held.join(" · ").to_string()],
                        ));
                    }
                    let off = fps(&|k| !row.contains(&k));
                    if !off.is_empty() {
                        lines.push(fill(
                            EN.public_pictures_leaves,
                            &[&off.join(" · ").to_string()],
                        ));
                    }
                    lines.push(EN.public_pictures_not_wallet.to_string());
                    lines.push(EN.public_pictures_public.to_string());
                    out.extend(pictures(
                        &format!("{stem}-share-{}-of-{n}", i + 1),
                        "-part-",
                        &text,
                        lines,
                    ));
                }
                out
            }
            // Specter DIY's `addwallet` command, in Specter's numbered
            // parts, which Specter DIY scans, past one code.
            14 => qr_view::specter_diy_pictures(c, w, shape, fps(&|_| true), &stem),
            _ => Vec::new(),
        }
    }

    /// One of a wallet's public files to the Outbox: 0 the blank
    /// template, 1 the descriptor, 2 the multisig config, 3 the backup
    /// sheet, 4 the split shares, 5 the wallet .json, 6 the BIP 129
    /// descriptor record, 7 Bitcoin Core's `importdescriptors` file, 8 the
    /// descriptor's code as a labelled picture; under More options
    /// ([`wallet_backup::public_more_files`]), 9 Bitcoin Core's import for Core before
    /// 29, 10 the Electrum wallet file, 11 the Caravan config, 12 Specter
    /// DIY's `addwallet` command, 13 the Wasabi file (`docs/INTEROP.md` §5
    /// I4, I8); and 14 that command's codes as labelled pictures, in
    /// Specter's numbered parts past one code.
    pub(crate) fn public_out(&mut self, wallet: usize, what: u8) {
        let files = self.public_made(wallet, what);
        let names: Vec<String> = files.iter().map(|(n, _)| n.clone()).collect();
        for (name, bytes) in files {
            self.put_outbox(&name, bytes);
        }
        self.backup_sent(wallet, &names);
        self.toast_out_files(&names);
    }

    /// Removes from For the stick what [`Faraday::public_out`] makes for
    /// `wallet` and `what`.
    fn public_remove(&mut self, wallet: usize, what: u8) {
        let names = self.public_names(wallet, what);
        let before = self.outbox.len();
        self.outbox.retain(|i| !names.contains(&i.name));
        if self.outbox.len() != before {
            self.save_boxes();
        }
    }

    /// The names of the files [`Faraday::public_made`] makes, with no
    /// picture or PDF made: a PDF that cannot be made is named anyway.
    fn public_names(&mut self, wallet: usize, what: u8) -> Vec<String> {
        self.public_files(wallet, what, false)
            .into_iter()
            .map(|(n, _)| n)
            .collect()
    }

    /// [`Faraday::public_made`], its pictures and PDFs made only when
    /// `bytes`, else named with no bytes.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn public_files(&mut self, wallet: usize, what: u8, bytes: bool) -> Vec<(String, Vec<u8>)> {
        let c = self.compute();
        let Some(w) = self.public_wallet(&c, wallet) else {
            return Vec::new();
        };
        let omit = self
            .backup
            .state()
            .map_or(Session::quorum(w).0.saturating_sub(1), |b| b.answers.omit);

        let stem = file_stem(&w.name);
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        match what {
            // The blank templates are printed from Faraday Starter: none is
            // made here. The sheets go out as PDFs, ready to print anywhere:
            // they hold nothing secret.
            1 => {
                files.push((
                    format!("{stem}-descriptor.txt"),
                    format!("{}\n", w.policy().to_descriptor_checksummed()).into_bytes(),
                ));
            }
            5 => files.push((
                format!("{stem}-wallet.json"),
                wallet_backup::wallet_json(&w.name, w.policy()).into_bytes(),
            )),
            6 => {
                if let Some(record) = self.wallet_file_text(&c, wallet, 6) {
                    files.push((format!("{stem}-bsms.txt"), record.into_bytes()));
                }
            }
            7 => files.push((
                format!("{stem}-bitcoin-core.json"),
                faraday_bip::core_import::import_descriptors(
                    &w.policy().to_descriptor_checksummed(),
                    faraday_bip::core_import::Rescan::Start,
                )
                .into_bytes(),
            )),
            2 => {
                if let Some(c) = self.wallet_file_text(&c, wallet, 2) {
                    files.push((format!("{stem}-multisig-config.txt"), c.into_bytes()));
                }
            }
            3 if !bytes => files.push((format!("{stem}-backup.pdf"), Vec::new())),
            3 => match pdf::sheet(&c, &wallet_backup::sheet_wallet(&c, &self.session, w)) {
                Ok(p) => files.push((format!("{stem}-backup.pdf"), p)),
                Err(e) => self.toast(&fill(EN.public_made_no_pdf, &[&e.to_string()])),
            },
            4 => {
                let (m, n) = Session::quorum(w);
                let plan = wallet_backup::split_plan(n, m, omit);
                let pictures = self.public_pictures(&c, wallet, 4);
                for (i, row) in plan.iter().enumerate() {
                    if let Some(share) = wallet_backup::split_share(w, row) {
                        // The sheet to print beside each file.
                        let pdf_name = format!("{stem}-share-{}-of-{n}.pdf", i + 1);
                        if !bytes {
                            files.push((pdf_name, Vec::new()));
                        } else if let Ok(pdf) = pdf::sheet(
                            &c,
                            &wallet_backup::sheet_share(w, i + 1, plan.len(), &share),
                        ) {
                            files.push((pdf_name, pdf));
                        }
                        files.push((
                            format!("{stem}-share-{}-of-{n}.txt", i + 1),
                            share.into_bytes(),
                        ));
                        // And its code, to scan back.
                        let share = format!("{stem}-share-{}-of-{n}", i + 1);
                        for p in pictures.iter().filter(|p| {
                            p.name
                                .strip_prefix(share.as_str())
                                .is_some_and(|rest| rest == ".png" || rest.starts_with("-part-"))
                        }) {
                            files
                                .push((p.name.clone(), if bytes { p.png(&c) } else { Vec::new() }));
                        }
                    }
                }
            }
            8 => {
                for p in self.public_pictures(&c, wallet, 8) {
                    files.push((p.name.clone(), if bytes { p.png(&c) } else { Vec::new() }));
                }
            }
            9 => {
                if let Some(c) = faraday_bip::core_import::import_descriptors_pair(w.policy()) {
                    files.push((
                        format!("{stem}-bitcoin-core-before-29.json"),
                        c.into_bytes(),
                    ));
                }
            }
            10 => {
                if let Some(c) = faraday_intake::electrum::file(w.policy()) {
                    files.push((format!("{stem}-electrum.json"), c.into_bytes()));
                }
            }
            11 => {
                let net = wallet::policy_network(w.policy(), self.session.network())
                    .unwrap_or(self.session.network());
                if let Some(c) = faraday_intake::caravan::file(&w.name, w.policy(), net) {
                    files.push((format!("{stem}-caravan.json"), c.into_bytes()));
                }
            }
            12 => {
                if let Some(c) = faraday_intake::specter_diy::command(&w.name, w.policy()) {
                    files.push((format!("{stem}-specter-diy.txt"), c.into_bytes()));
                }
            }
            14 => {
                for p in self.public_pictures(&c, wallet, 14) {
                    files.push((p.name.clone(), if bytes { p.png(&c) } else { Vec::new() }));
                }
            }
            13 => {
                if let Some(c) = faraday_intake::single_key::wasabi_file(w.policy()) {
                    files.push((format!("{stem}-wasabi.json"), c.into_bytes()));
                }
            }
            _ => {}
        }
        files
    }

    /// Costs what `account_text` costs. The account key in slot `slot` of
    /// wallet `i`, when this session holds its seed: for the cosigners of a
    /// wallet of more than one key. `None` for a silent payments wallet,
    /// whose slot is not an account key.
    pub fn wallet_key(&self, c: &Compute, i: usize, slot: u8) -> Result<WalletKey, String> {
        let w = self
            .session
            .wallets
            .get(i)
            .ok_or(EN.wallet_key_that_wallet)?;
        if w.policy().silent().is_some() {
            return Err(EN.wallet_key_this_wallet_s.into());
        }
        let pk = w
            .policy()
            .keys()
            .get(usize::from(slot))
            .ok_or(EN.wallet_key_this_wallet)?;
        let fp = pk.fingerprint().ok_or(EN.wallet_key_that_key_names)?;
        let key = self
            .session
            .keys
            .position(fp)
            .ok_or(EN.wallet_key_that_key)?;
        let kind = create::NewKind::of(w.policy());
        // Create's own account for its kind: what its BIP 129 record and
        // its account code are made at. A key elsewhere has neither.
        let standard = kind.filter(|k| {
            self.session
                .keys
                .account_text(c, key, *k, 0)
                .ok()
                .and_then(|t| faraday_bip::policy::PolicyKey::parse(&t).ok())
                .is_some_and(|t| t.xpub() == pk.xpub() && t.path() == pk.path())
        });
        Ok(WalletKey {
            fp: fp_text(fp),
            text: pk.key_text(),
            kind: kind.map_or_else(|| Session::shape(w), |k| k.name().to_string()),
            standard,
            key,
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
        for slot in 0..w.policy().keys().len().min(usize::from(u8::MAX)) {
            let slot = slot as u8;
            if let Some(fp) = self.wallet_key_here(i, slot)
                && !seen.contains(&fp)
            {
                seen.push(fp);
                out.push(slot);
            }
        }
        out
    }

    /// The master fingerprint of slot `slot` of wallet `i` when
    /// [`Faraday::wallet_key`] has that key: its seed is here and the
    /// wallet is no silent payments wallet. A lookup; it
    /// derives nothing.
    fn wallet_key_here(&self, i: usize, slot: u8) -> Option<faraday_bip::keys::Fingerprint> {
        let w = self.session.wallets.get(i)?;
        if w.policy().silent().is_some() {
            return None;
        }
        let fp = w.policy().keys().get(usize::from(slot))?.fingerprint()?;
        self.session
            .keys
            .all()
            .any(|k| k.fingerprint == fp)
            .then_some(fp)
    }

    /// Costs what `wallet_key` and `bsms_record` cost. A wallet key's BIP
    /// 129 key record, signed by the key: for a `wsh` or `sh(wsh)` multisig
    /// key at Create's own account.
    fn wallet_key_record(
        &self,
        c: &Compute,
        i: usize,
        slot: u8,
    ) -> Result<(String, String), String> {
        let k = self.wallet_key(c, i, slot)?;
        k.standard
            .and_then(|kind| kind.bsms_record(c, &self.session.keys, k.key, 0, &k.fp))
            .map(|r| (k.fp.clone(), r))
            .ok_or_else(|| EN.wallet_key_record_bip.to_string())
    }

    fn wallet_key_out(&mut self, i: usize, slot: u8) {
        let c = self.compute();
        match self.wallet_key(&c, i, slot) {
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
        let c = self.compute();
        match self.wallet_key_record(&c, i, slot) {
            Ok((fp, text)) => {
                let name = format!("xpub-{fp}-bsms.txt");
                self.put_outbox(&name, text.into_bytes());
                self.backup_sent(i, std::slice::from_ref(&name));
                self.toast_out(&name);
            }
            Err(e) => self.toast(&e),
        }
    }

    fn make_pdf(&mut self, name: &str, bytes: &[u8]) {
        let c = self.compute();
        if !self.online {
            return;
        }
        match pdf::sheet(&c, &String::from_utf8_lossy(bytes)) {
            Ok(pdf) => self.out.storage_out.push_back(StorageCommand::Print {
                name: fill(EN.make_pdf_pdf, &[&stem(name).to_string()]),
                bytes: pdf,
            }),
            Err(e) => self.toast(&fill(EN.make_pdf_not_printed, &[&e.to_string()])),
        }
    }

    /// Costs an address derivation (through `find_address`). What a scanned
    /// address is to the loaded wallets: one of a wallet's first hundred on
    /// either chain, or none of them (`docs/QR.md` §2: "Is this address
    /// mine?"). `None` when the text is not an address.
    fn address_note(&self, c: &Compute, text: &str) -> Option<String> {
        let t = text.trim();
        let t = t.strip_prefix("bitcoin:").unwrap_or(t);
        let t = t.split('?').next().unwrap_or(t);
        let net = self.session.network();
        let Ok(addr) =
            t.parse::<faraday_bip::bitcoin::Address<faraday_bip::bitcoin::address::NetworkUnchecked>>()
        else {
            // An address's characters, as many as one has, that fail its
            // checksum: one of them is wrong.
            let whole = t.len() >= 26
                && faraday_bip::keys::Network::ALL
                    .iter()
                    .any(|&n| faraday_bip::address::is_address_prefix(t, n));
            return whole.then(|| EN.address_note_that_address_fails.to_string());
        };
        let Ok(addr) = addr.require_network(net.into()) else {
            return Some(fill(
                EN.address_note_wrong_network,
                &[wallet::network_name(net)],
            ));
        };
        for w in &self.session.wallets {
            if let Some((change, index)) = w.policy().find_address(c, net, &addr, 100) {
                let chain = if change {
                    EN.address_note_change
                } else {
                    EN.address_note_receive
                };
                return Some(fill(
                    EN.address_note_found,
                    &[&w.name.to_string(), chain, &index.to_string()],
                ));
            }
        }
        Some(if self.session.wallets.is_empty() {
            EN.address_note_no_wallet.to_string()
        } else {
            EN.address_note_that_address.to_string()
        })
    }

    fn open_qr(&mut self, view: Result<QrView, String>) {
        match view {
            Ok(v) => self.qr.open(v, &mut self.sheet),
            Err(e) => self.toast(&fill(EN.open_qr_cannot_make, &[&e.to_string()])),
        }
    }

    /// A view's code as a labelled picture to the Outbox, when it is one
    /// code of public content.
    fn code_png(&mut self, view: Result<&QrView, &String>) {
        let c = self.compute();
        let v = match view {
            Ok(v) => v,
            Err(e) => return self.toast(&fill(EN.code_png_cannot_make, &[&e.to_string()])),
        };
        match v.png(&c) {
            Some((name, bytes)) => {
                self.put_outbox(&name, bytes);
                self.toast_out(&name);
            }
            None if v.public && v.frames.len() > 1 => {
                self.toast(&fill(EN.code_png_is_more_than, &[&v.title.to_string()]));
            }
            None => {}
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
            .policy()
            .keys()
            .iter()
            .filter_map(|k| k.fingerprint().map(fp_text))
            .collect();
        if !all.is_empty() {
            let word = if all.len() == 1 {
                EN.wallet_label_key
            } else {
                EN.wallet_label_keys
            };
            lines.push(format!("{word} {}", all.join(" · ")));
        }
        lines
    }

    /// Costs an address derivation (through `address_at`), or `checked_config`.
    /// A wallet's public text file `what` (as [`Faraday::public_out`] numbers
    /// them): 2 the multisig config where it rebuilds the wallet, 6 BIP 129's.
    fn wallet_file_text(&self, c: &Compute, wallet: usize, what: u8) -> Option<String> {
        let w = self.session.wallets.get(wallet)?;
        match what {
            2 => restore::checked_config(c, self.session.network(), w),
            6 => {
                let net = self.session.network();
                let first = w.policy().address_at(c, net, false, 0).ok()?;
                Some(
                    faraday_bip::bsms::DescriptorRecord {
                        policy: w.policy().clone(),
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
        match self.entry.state().back {
            Some(s @ (Screen::Family | Screen::Spend | Screen::BackupFlow | Screen::Create)) => s,
            Some(Screen::Restore) if self.restore.state().is_some_and(|r| r.seeds.is_some()) => {
                Screen::Restore
            }
            _ => Screen::Wallets,
        }
    }

    /// Loads a key typed or scanned (not made here: no `mark_made_here`):
    /// the words with `pass`, as the key `wanted` if one is (a key
    /// already loaded refused as a duplicate); `said` says so; then what
    /// the screen it returns to, `back`, does with it (`seeds_took`).
    /// Add a key's Add and New key's Lock in for a key typed both load
    /// through here (`docs/history/NEW-WALLET.md` §15.2).
    pub(crate) fn typed_load(
        &mut self,
        m: &faraday_bip::bip39::Mnemonic,
        pass: &str,
        label: &str,
        wanted: Option<faraday_bip::keys::Fingerprint>,
        back: Option<Screen>,
        said: impl FnOnce(&mut Self, faraday_bip::keys::Fingerprint),
    ) -> Result<faraday_bip::keys::Fingerprint, wallet::Refusal> {
        let c = self.compute();
        let fp = self.session.keys.add_mnemonic(&c, m, pass, label, wanted)?;
        said(self, fp);
        self.seeds_took(fp, back);
        Ok(fp)
    }

    /// Enter on the words form: Add a key adds the key; New key's Words
    /// card goes on to Key.
    fn entry_enter(&mut self) {
        if self.screen == Screen::KeyGen {
            self.keygen_act(Action::KNext);
        } else {
            self.entry_add();
        }
    }

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn entry_add(&mut self) {
        let c = self.compute();
        if !self.may_load_keys() {
            return;
        }
        let wanted = self
            .entry
            .state()
            .wanted
            .map(faraday_bip::keys::Fingerprint);
        let label = self.entry.state().label.clone();
        let typed = self.entry.mnemonic();
        let m = match typed {
            Ok(m) => m,
            Err(e) => {
                self.entry.state_mut().error = Some(e);
                return;
            }
        };
        // A SeedQR scanned over the Spend tab or Restore returns to it too.
        let back =
            self.entry
                .state()
                .back
                .or(matches!(self.screen, Screen::Family | Screen::Restore).then_some(self.screen));
        // Add a key for a wallet takes only a key the wallet is missing
        // (`docs/history/SCAN-TO-SIGN.md` §2.1).
        let mut label = label;
        let mut for_wallet = None;
        if let Some(w) = self.entry.state().wallet {
            let fp = faraday_keys::identity::fingerprint_of_mnemonic(
                &c,
                &m,
                &self.entry.state().passphrase,
            )
            .ok();
            if let Some(fp) = fp {
                match self.wallet_key_found(w, fp) {
                    scan_key::Found::None => {
                        let wallet = self
                            .session
                            .wallets
                            .get(w)
                            .map(|wl| wl.name.clone())
                            .unwrap_or_default();
                        self.entry.state_mut().error = Some(
                            wallet::Refusal::NotOfWallet {
                                got: fp_text(fp),
                                wallet,
                            }
                            .text(),
                        );
                        return;
                    }
                    scan_key::Found::Missing(n) => {
                        if label.trim().is_empty() {
                            label = self.wallet_key_label(&c, w, fp);
                        }
                        for_wallet = Some((w, n));
                    }
                    scan_key::Found::Here(_) => {}
                }
            }
        }
        // Restore takes a seed only into a slot it fills: with the
        // description known, one of its keys (`docs/history/NEW-WALLET.md` §12.1).
        if back == Some(Screen::Restore)
            && let Some(why) = self.restore_refusal_typed(&c, &m)
        {
            if let Some(r) = self.restore.state_mut() {
                r.error = Some(why);
            }
            self.entry.reset();
            self.screen = Screen::Restore;
            return;
        }
        let pass = secret_text::SecretText::of(self.entry.state().passphrase.as_str());
        let said = |app: &mut Self, fp: faraday_bip::keys::Fingerprint| match for_wallet {
            Some((w, n)) => app.wallet_key_said(w, n),
            None => app.toast(&fill(EN.entry_add_key_added, &[&fp_text(fp).to_string()])),
        };
        match self.typed_load(&m, &pass, &label, wanted, back, said) {
            Ok(_) => {
                self.entry.reset();
                self.refresh_spend();
                self.screen = match back {
                    Some(s) => s,
                    None if self.wallets.spend().is_some() => Screen::Spend,
                    None => Screen::Wallets,
                };
                if back == Some(Screen::Family) {
                    self.family_key_added();
                }
            }
            // A seed already loaded comes in on the Spend tab's What do
            // you have? as it is (`docs/history/SCAN-TO-SIGN.md` §4).
            Err(wallet::Refusal::Duplicate(_)) if back == Some(Screen::Family) => {
                if let Ok(fp) = faraday_keys::identity::fingerprint_of_mnemonic(
                    &c,
                    &m,
                    &self.entry.state().passphrase,
                ) {
                    self.seeds_took(fp, back);
                    self.entry.reset();
                    self.refresh_spend();
                    self.screen = Screen::Family;
                    self.family_key_added();
                }
            }
            // A SeedQR scanned on Restore: the refusal shows there.
            Err(e) if self.screen == Screen::Restore => {
                if let Some(r) = self.restore.state_mut() {
                    r.error = Some(e.text());
                }
                self.entry.reset();
            }
            Err(e) => self.entry.state_mut().error = Some(e.text()),
        }
    }

    /// Loads the key an Inbox file's words spell, the same as typing them
    /// into Add a key. Refused with no stick attached.
    fn load_key(&mut self, index: usize) -> bool {
        let c = self.compute();
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
        match self.session.keys.add_words(&c, &words, &label, None) {
            Ok(fp) => {
                self.toast(&fill(EN.load_key_key_added, &[&fp_text(fp).to_string()]));
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
        let c = self.compute();
        if payloads.is_empty() {
            self.visit
                .log
                .push((fill(EN.qr_read_copied_no_qr, &[name]), true));
        } else {
            // An image is read as the camera reads: a UR in parts can
            // span several images.
            if self.camera.scan().is_none() {
                self.camera.open(ScanState::default());
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
                    self.inbox
                        .push(Item::new(&c, &got, words.as_bytes().to_vec()));
                    self.visit.load_after.insert(got.clone());
                    self.save_boxes();
                    seeds += 1;
                    self.visit.log.push((
                        fill(
                            EN.qr_read_a_seedqr_copied,
                            &[name, &got.to_string(), self.medium.noun()],
                        ),
                        true,
                    ));
                    continue;
                }
                if self.camera.scan().is_none() {
                    self.camera.open(ScanState::default());
                }
                let parts = self.camera.scan().is_some_and(ScanState::in_parts);
                let n = self.inbox.len();
                self.scanned(p);
                // A transfer in parts that has just come together
                // ends the reading: what follows is another.
                if parts && self.inbox.len() > n {
                    break;
                }
            }
            let note = self.camera.scan().and_then(|s| s.note.clone());
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
                self.toast(&fill(EN.qr_read_is_in_files, &[&got.to_string()]));
                self.visit.log.push((
                    fill(EN.qr_read_read_into_files, &[name, &got.to_string()]),
                    true,
                ));
            } else if seeds == 0 {
                self.visit.log.push((
                    format!("{name}: {}", note.unwrap_or_else(|| EN.qr_read_read.into())),
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

    /// Whether the stick on show holds the Inbox file `item`, by name and
    /// size.
    fn visit_stick_holds(&self, item: &Item) -> bool {
        self.stick.get(self.visit.stick).is_some_and(|s| {
            s.files
                .iter()
                .any(|(n, len)| *n == item.name && *len == item.bytes.len() as u64)
        })
    }

    /// The Inbox files a stick visit's left column shows, by index, each
    /// with whether the stick on show holds it already (by name and size):
    /// those rows are drawn greyed and cannot be ticked.
    pub fn visit_inbox_shown(&self) -> Vec<(usize, bool)> {
        self.inbox
            .iter()
            .enumerate()
            .map(|(k, i)| (k, self.visit_stick_holds(i)))
            .collect()
    }

    /// The Inbox files a stick visit offers to write, by index: every one
    /// but those the stick shown holds already, by name and size.
    pub fn visit_inbox_rows(&self) -> Vec<usize> {
        self.visit_inbox_shown()
            .into_iter()
            .filter(|&(_, held)| !held)
            .map(|(k, _)| k)
            .collect()
    }

    /// Whether the Inbox holds a file of this name and size: the stick
    /// row says "In Faraday".
    pub fn inbox_holds(&self, name: &str, size: u64) -> bool {
        self.inbox
            .iter()
            .any(|i| i.name == name && i.bytes.len() as u64 == size)
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
            (Some(FileKind::Other), _) => EN.visit_inbox_png_no_qr.to_string(),
            (Some(k), _) => fill(
                EN.visit_inbox_png_kind,
                &[&screens::kind_name(k).to_lowercase().to_string()],
            ),
            (None, FileKind::Other) => EN.visit_inbox_a_file.to_string(),
            (None, k) => screens::kind_name(k).to_string(),
        };
        let gives = match holds {
            FileKind::Words => EN.visit_inbox_whoever_has_it_holds,
            FileKind::SeedPart => EN.visit_inbox_whoever_has_enough,
            FileKind::Entries => EN.visit_inbox_vault_warning,
            _ => EN.visit_inbox_whoever,
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
        if let Some(stick) = self.stick.get(self.visit.stick) {
            let id = stick.id.clone();
            self.visit.log.clear();
            for name in self.visit.inn.clone() {
                // A picture's QR codes are read, not its bytes.
                let stick = id.clone();
                self.visit.reading.insert(name.clone());
                self.out
                    .storage_out
                    .push_back(if name.to_ascii_lowercase().ends_with(".png") {
                        StorageCommand::ReadQr { stick, name }
                    } else {
                        StorageCommand::Read { stick, name }
                    });
            }
        }
    }

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn sign_here(&mut self) {
        let c = self.compute();
        // A key made here signs nothing until its backup is done
        // (`docs/history/NEW-WALLET.md` §14.3).
        let held = self.held_keys_now(&c);
        let waiting: Vec<String> = self
            .wallets
            .spend()
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
            if let Some(s) = self.wallets.spend_mut() {
                s.error = Some(fill(
                    EN.sign_here_backup_first,
                    &[&waiting.join(", ").to_string()],
                ));
                s.open = Some(wallet::step::SIGNERS);
                s.follow = true;
            }
            return;
        }
        // A MuSig2 round opened here draws its secret nonces
        // from a seed of this pass's own; a pass with no such input
        // draws nothing and needs none.
        let opens_round = self
            .wallets
            .spend()
            .is_some_and(|s| spend_opens_round(&s.inspection));
        let seed = match self.sign_seed() {
            Some(seed) => seed,
            None if opens_round => {
                if let Some(s) = self.wallets.spend_mut() {
                    s.error = Some(EN.sign_here_no_randomness.to_string());
                }
                return;
            }
            None => zeroize::Zeroizing::new([0u8; 32]),
        };
        // The same transaction stating other amounts than when it was
        // signed here is refused, with no override (`docs/WALLETS.md` §3.3).
        if self
            .wallets
            .spend()
            .is_some_and(|s| self.amounts_refused(&s.spend.psbt))
        {
            if let Some(s) = self.wallets.spend_mut() {
                s.error = Some(EN.sign_here_this_transaction.to_string());
            }
            return;
        }
        let mut remember: Option<faraday_psbt::Psbt> = None;
        let Some(s) = self.wallets.spend_mut() else {
            return;
        };
        let before = s.spend.psbt.to_bytes();
        let mut psbt = s.spend.psbt.clone();
        let mut nonce_only = false;
        let signed = self.session.sign_spend(&c, &mut psbt, &seed, s);
        // A session whose every nonce has signed is over: the next pass
        // over this transaction opens a new one, never the same nonce.
        if s.musig.as_ref().is_some_and(|m| m.is_empty()) {
            s.musig = None;
        }
        let signed = signed.map(|out| out.by);
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
            if let Some(s) = self.wallets.spend_mut() {
                s.open = Some(wallet::step::NONCES);
                s.follow = true;
            }
            self.toast(EN.sign_here_this_device);
            return;
        }
        self.refresh_spend();
        if self
            .wallets
            .spend()
            .is_some_and(|s| s.spend.psbt.to_bytes() != before)
        {
            self.part_out();
        }
        if let Some(s) = self.wallets.spend_mut() {
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
        let c = self.compute();
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        let bytes = item.bytes.clone();
        let name = item.name.clone();
        // Drawn before the spend is borrowed: a MuSig2 aggregation below
        // may still share a nonce, and never the one shared before.
        let seed = self.sign_seed();
        let Some(s) = self.wallets.spend_mut() else {
            return;
        };
        let before = s.spend.psbt.to_bytes();
        match s.spend.collect(&c, &self.session, &bytes, &name) {
            Ok(added) if added.is_empty() && s.spend.psbt.to_bytes() == before => {
                s.error = Some(fill(EN.collect_has_nothing, &[&name.to_string()]));
            }
            Ok(_) => s.error = None,
            Err(e) => s.error = Some(e),
        }
        // A MuSig2 wallet: once every partial signature is in, this device
        // aggregates them; until then the pass changes nothing.
        let musig = self
            .session
            .wallet_for(&c, &s.spend.psbt)
            .and_then(|i| self.session.wallets.get(i))
            .is_some_and(|w| wallet::Kind::of(w.policy()) == wallet::Kind::MuSig);
        if musig && !s.spend.signed_here.is_empty() && s.spend.finish().is_err() {
            let mut psbt = s.spend.psbt.clone();
            match &seed {
                Some(seed) => {
                    if self.session.sign_spend(&c, &mut psbt, seed, s).is_ok() {
                        s.spend.psbt = psbt;
                    }
                    if s.musig.as_ref().is_some_and(|m| m.is_empty()) {
                        s.musig = None;
                    }
                }
                None => {
                    s.error = Some(EN.collect_no_randomness.to_string());
                }
            }
        }
        self.refresh_spend();
        if self
            .wallets
            .spend()
            .is_some_and(|s| s.spend.psbt.to_bytes() != before)
        {
            self.part_out();
        }
        if let Some(s) = self.wallets.spend_mut() {
            s.follow = true;
            if s.complete {
                s.done[wallet::step::COLLECT as usize] = true;
                s.open = Some(wallet::step::FINISH);
            }
        }
    }

    fn primary(&mut self) {
        let Some(s) = self.wallets.spend_mut() else {
            return;
        };
        let complete = s.complete;
        let can_sign_here = s
            .inspection
            .participating_keys
            .iter()
            .any(|fp| self.session.keys.key_label(*fp).is_some() && !s.signers.contains(&fp.0));
        if s.spend.finished.is_some() {
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
                s.error = Some(fill(EN.primary_finish_step, &[&(pos + 1).to_string()]));
            } else {
                self.sign_here();
            }
        } else {
            s.open = Some(wallet::step::COLLECT);
            s.follow = true;
        }
    }

    /// Works out the key a passphrase typed makes, once typing has paused
    /// for [`motion::TYPED_PAUSE_MS`]: the potential wallet's, or the
    /// seed read's; or lets the next frame work out the key the words
    /// typed into Add a key make ([`EntryState::probed`]). Returns whether
    /// there is something new to show.
    fn typed_tick(&mut self, now_ms: u64) -> bool {
        let Some(at) = self.inbox.typed_at() else {
            return false;
        };
        if now_ms < at + motion::TYPED_PAUSE_MS {
            return false;
        }
        self.inbox.set_typed_at(None);
        match self.sheet {
            Some(Sheet::Potential) => self.potential_refresh(),
            Some(Sheet::SeedRead) => self.seed_read_typed(),
            _ => return self.entry_typing_here(),
        }
        true
    }

    /// Whether typing has not paused since the last character
    /// ([`motion::TYPED_PAUSE_MS`]): a key the typing makes is not worked
    /// out yet.
    pub(crate) fn typing_now(&self) -> bool {
        self.inbox.typed_at().is_some()
    }

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn key(&mut self, key: KeyIn) {
        // Escape takes a tooltip off first, and does nothing else.
        if key == KeyIn::Escape && self.pointer.tip_dismiss() {
            return;
        }
        if self.selftest_failed() {
            return;
        }
        // Ctrl+V is the scanner's Paste, or Paste words on the words form.
        if key == KeyIn::Paste {
            return self.paste();
        }
        if self.learn_key(key) || self.keygen_walk_key(key) || self.scroll_key(key) {
            return;
        }
        if self.board_key(key) || self.gpg_restore_key(key) {
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
        if self.file_key(key) || self.vaults.search_key(self.screen, key) {
            return;
        }
        if self.seed_read_key(key) || self.sweep_key(key) {
            return;
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
        if self.bip85_key(key) || self.keygen_key(key) || self.create_pass_key(key) {
            return;
        }
        if self.set_time_key(key) || self.own_key(key) {
            return;
        }
        if self.screen == Screen::Entry && self.entry.state().on_passphrase {
            match self.entry.passphrase_key(key) {
                state::Keyed::Add => self.entry_add(),
                state::Keyed::Recover => self.form_recover(),
                _ => {}
            }
            self.entry.clear_error();
            return;
        }
        let here = self.entry_typing_key();
        if here && self.entry.state().form != forms::Form::Words {
            match self.entry_part_key(key) {
                state::Keyed::Recover => {
                    if self.screen == Screen::KeyGen {
                        self.keygen_act(Action::KNext);
                    } else {
                        self.form_recover();
                    }
                }
                state::Keyed::AddPart => self.form_add_part(),
                state::Keyed::Leave => {
                    let to = self.entry_leave();
                    self.back();
                    if self.screen == Screen::Entry {
                        self.screen = to;
                    }
                }
                _ => {}
            }
            return;
        }
        // The words of a key: a list with an on-screen keyboard takes what
        // a physical one can type of it (pinyin's letters and tone digits,
        // or the list's own characters from an input method); the rest are
        // typed in Latin letters. Check my copy types through the same
        // function (`forms::word_key`).
        if here {
            let e = self.entry.state_mut();
            let lang = e.language();
            let typed = forms::word_key(
                forms::WordsMut {
                    typed: &mut e.typed,
                    keys: &mut e.keys,
                    error: &mut e.error,
                    lang,
                    numbers: false,
                },
                key,
            );
            match typed {
                forms::WordKey::Enter => self.entry_enter(),
                forms::WordKey::Escape => self.screen = self.entry_leave(),
                _ => {}
            }
            return;
        }
        if self.screen == Screen::Wallets
            && let Some(name) = self.wallets.renaming_mut()
        {
            match key {
                KeyIn::Char(c) if name.chars().count() < 40 => name.push(c),
                KeyIn::Backspace => {
                    name.pop();
                }
                KeyIn::Enter => {
                    let n = wallet::wallet_name(name).into_owned();
                    if !n.is_empty()
                        && let Some(w) = self.session.wallets.get_mut(self.wallets.wallet())
                    {
                        w.name = n;
                    }
                    self.wallets.set_renaming(None);
                }
                KeyIn::Escape => self.wallets.set_renaming(None),
                _ => {}
            }
            return;
        }
        if self.screen == Screen::Message && self.message.key(key) {
            return;
        }
        // The words of a copy, typed back for Check my copy.
        if self.screen == Screen::Backup && self.backup.state().is_some_and(|b| b.check.is_some()) {
            self.copy_check_key(key);
            return;
        }
        // A place's name, a codex32 string, the copy grid's mark.
        if self.backup_name_key(key) {
            return;
        }
        match key {
            KeyIn::Escape => {
                // Escape is the sheet's Cancel: the scanner's camera goes
                // off with it, as on Cancel.
                if self.sheet.is_some() && self.sheet != Some(Sheet::Lock) {
                    self.act(Action::Cancel);
                }
            }
            KeyIn::Down => self.glide(motion::WHEEL_NOTCH),
            KeyIn::Up => self.glide(-motion::WHEEL_NOTCH),
            // What no screen took moves the keyboard's place, or presses
            // the control it is on.
            _ => self
                .pointer
                .focus_key(key)
                .into_iter()
                .for_each(|a| self.press_in(a)),
        }
    }

    /// The offset of the region that scrolls now, in design units: the
    /// open sheet's, the screen's, or another region of the screen's
    /// that the pointer went to ([`Faraday::slot`]).
    fn scroll_slot(&mut self) -> Option<&mut f32> {
        match self.slot() {
            ui::Slot::VisitOut => Some(&mut self.visit.out_offset),
            ui::Slot::Card => Some(self.wallets.card_offset_mut()),
            ui::Slot::VaultItem => Some(self.vaults.item_offset_mut()),
            ui::Slot::FilesOut => Some(self.visit.files_out_offset_mut()),
            s @ (ui::Slot::ListPane | ui::Slot::Sidebar) => self.scroll.offset_of(s),
            ui::Slot::BoardList => Some(self.flows.board_list_offset_mut()),
            ui::Slot::LearnPane => Some(self.learn.scroll_mut()),
            ui::Slot::Worksheet | ui::Slot::DemoPane | ui::Slot::InputPanel => self.codex32_slot(),
            ui::Slot::Page | ui::Slot::WorksheetWhole | ui::Slot::DiceWork => self.page_slot(),
        }
    }

    /// Which region scrolls now: the one the pointer last went to, while
    /// it can be reached; else the screen's or the sheet's own.
    fn slot(&self) -> ui::Slot {
        self.scroll
            .slot(self.screen, self.sheet.is_none(), self.compact, || {
                self.board_shown()
            })
    }

    /// The regions the last frame drew that can be reached now, last
    /// drawn first: through a sheet that does not scroll, none.
    fn regions(&self) -> std::vec::IntoIter<ui::Scrolled> {
        let key = self.region_key();
        self.scroll
            .regions(key, self.sheet.is_none() || key.1.is_some())
    }

    /// What the region that scrolls now rounds an offset to: whole
    /// pixels, and never past the end the last frame drew. Rounded up
    /// past it, the page would clamp it back and ask for another frame,
    /// for ever at a scale that is not whole.
    fn whole_offset(&self) -> impl Fn(f32) -> f32 + use<> {
        let f = self.f.max(0.1);
        let end = self.region_now().map(|e| (e.max * f).floor().max(0.0));
        move |o| {
            let px = (o * f).round();
            end.map_or(px, |end| px.min(end)) / f
        }
    }

    /// The region that scrolls now, as the last frame drew it.
    fn region_now(&self) -> Option<ui::Scrolled> {
        self.scroll.region_now(self.region_key(), self.slot())
    }

    /// The wheel, a trackpad or a finger starts at (x, y): the region
    /// under it is the one they move (the innermost, where one region
    /// stands inside another), the page's when none is.
    fn point_region(&mut self, x: i32, y: i32) {
        let slot = self
            .regions()
            .filter(|e| e.view.contains(x, y))
            .min_by_key(|e| i64::from(e.view.w) * i64::from(e.view.h))
            .map_or(ui::Slot::Page, |e| e.slot);
        self.scroll.went_to(slot);
    }

    /// Which region scrolls now: the screen, or the sheet over it when
    /// that sheet scrolls. A sheet that does not scroll leaves the screen
    /// the region, so what moves on it carries on under the sheet.
    fn region_key(&self) -> ScreenKey {
        (self.screen, self.sheet)
    }

    /// The motion belongs to the region that scrolls now: what moved
    /// another screen, sheet or region is stopped.
    fn own_motion(&mut self) {
        let key = (self.region_key(), self.slot());
        self.scroll.own(key);
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
        let mut m = self.scroll.take_motion(self.reduce_motion());
        let out = self.scroll_slot().map(|offset| {
            let mut region = motion::Region {
                offset,
                max: extent.map(|e| e.max),
                view: extent.map_or(h, |e| e.view.h as f32 / f),
                f,
            };
            go(&mut m, &mut region)
        });
        self.scroll.put_back(m);
        out
    }

    /// Finds what can be pressed under the pointer in the frame on
    /// screen. Returns whether that changed.
    fn find_hovered(&mut self) -> bool {
        self.pointer.find_hovered()
    }

    /// Fingers moved the content by `dy` units: at once.
    fn pan(&mut self, dy: f32) {
        self.with_region(|m, r| m.pan(r, dy));
    }

    /// A wheel or a key asked for `dy` units: the content glides there.
    fn glide(&mut self, dy: f32) {
        if self.reduce_motion() {
            self.with_region(|m, r| m.jump(r, dy));
        } else if self.scroll_slot().is_some() {
            // A glide on the region the pointer has just moved to starts
            // there, not on the region that was moving.
            self.own_motion();
            self.scroll.glide(dy);
        }
    }

    /// Moves whatever is moving on to `now_ms`. Returns whether a frame
    /// is wanted for it.
    fn motion_tick(&mut self, now_ms: u64) -> bool {
        match self.with_region(|m, r| m.tick(r, now_ms)) {
            Some(moved) => moved,
            None => {
                self.scroll.stop();
                false
            }
        }
    }

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn touch(&mut self, x: u16, y: u16, phase: TouchPhase) {
        let (x, y) = (i32::from(x), i32::from(y));
        let found = self.pointer.hit_at(x, y);
        match phase {
            TouchPhase::Down => {
                self.pointer.set_down_at((x, y));
                self.pointer.tip_down((x, y), self.now_ms);
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
                    self.pointer.set_bar_held(Some(grab));
                    self.scroll.stop();
                    self.pointer.set_pressed(None);
                    self.pointer.set_drag(None);
                    self.pointer.set_swallow(false);
                    self.bar_drag(y);
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                    return;
                }
                // A finger on a scrolled region may be the start of a
                // drag; it is a press until it moves. Not through a sheet
                // that does not scroll: the page under it is out of reach.
                let slot = self.slot();
                let dragging = self
                    .regions()
                    .find(|e| e.slot == slot && e.view.contains(x, y))
                    .map(|_| (y, false));
                self.pointer.set_drag(dragging);
                // A touch on content that is still flying only stops it.
                self.pointer.set_swallow(self.scroll.flung(self.f));
                if self.pointer.swallow() {
                    self.scroll.stop();
                    self.pointer.set_pressed(None);
                    return;
                }
                self.pointer.set_pressed(found);
                self.vaults.pressed(self.now_ms);
                // A press on a chip of the backup board may be its drag:
                // the page does not scroll under it (§19.2).
                if self.board_down(found.map(|f| f.0), x, y) {
                    self.pointer.set_drag(None);
                }
                if let Some((Action::VisitBar(c), _)) = found {
                    self.visit_drag(c, y);
                }
            }
            TouchPhase::Move => {
                self.pointer.tip_moved((x, y));
                if self.pointer.bar_held().is_some() {
                    self.bar_drag(y);
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                    return;
                }
                // A board chip past the slop follows the finger.
                if self.board_move(x, y) {
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                    return;
                }
                let held_bar = match self.pointer.pressed() {
                    Some((Action::VisitBar(c), _)) => Some(c),
                    _ => None,
                };
                // A slider held follows the finger along its track.
                if let Some((Action::Slide(id, _), r)) = self.pointer.pressed() {
                    let row = r.y + r.h / 2;
                    let under = self
                        .pointer
                        .hits()
                        .iter()
                        .rev()
                        .find(|(h, a)| {
                            matches!(a, Action::Slide(i, _) if *i == id) && h.contains(x, row)
                        })
                        .map(|(h, a)| (*a, *h));
                    if let Some((a, h)) = under
                        && Some(a) != self.pointer.pressed().map(|p| p.0)
                    {
                        self.pointer.set_pressed(Some((a, h)));
                        self.act(a);
                        self.out.dirty = true;
                        self.out.commands.push_back(Command::Draw);
                    }
                    return;
                }
                if let Some((last, dragging)) = self.pointer.drag()
                    && held_bar.is_none()
                {
                    let slop = motion::SLOP * self.f;
                    let (dx, dy) = (x - self.pointer.down_at().0, y - self.pointer.down_at().1);
                    let start = !dragging && self.drag_starts(dx, dy, slop);
                    if dragging || start {
                        // Past the slop the press is a drag: nothing
                        // under the finger is pressed, the page follows.
                        self.pointer.set_pressed(None);
                        self.pointer.set_drag(Some((y, true)));
                        self.pan_drag((x, y), last, start);
                        self.out.dirty = true;
                        self.out.commands.push_back(Command::Draw);
                        return;
                    }
                }
                // The scrollbar follows the finger anywhere once held.
                if let Some(c) = held_bar {
                    self.visit_drag(c, y);
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                } else if let Some((_, r)) = self.pointer.pressed()
                    && !r.contains(x, y)
                {
                    self.pointer.set_pressed(None);
                }
            }
            TouchPhase::Up => {
                self.pointer.tip_up();
                if self.pointer.take_bar_held().is_some() {
                    return;
                }
                // A board chip dragged is dropped where it is lifted.
                if self.board_up(x, y) {
                    self.pointer.set_drag(None);
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                    return;
                }
                let dragged = self.pointer.take_drag().is_some_and(|(_, d)| d);
                if dragged {
                    self.pointer.set_pressed(None);
                    self.scroll.release(self.f);
                }
                if dragged || self.pointer.take_swallow() {
                    return;
                }
                let pressed = self.pointer.take_pressed();
                // A hold acts on the tick that completes it, never on release.
                if let (Some((a, _)), Some((b, _))) = (pressed, found)
                    && a == b
                    && !vault::is_hold(a)
                {
                    // On a field: the caret where it was pressed, a
                    // second press selecting all of it, a drag what it
                    // went over (`docs/history/NEW-WALLET.md` §13.2).
                    let double = self
                        .pointer
                        .last_tap()
                        .is_some_and(|(l, t)| l == a && self.now_ms.saturating_sub(t) <= 450);
                    if field_action(a)
                        && let Some(dragged) =
                            self.field_press(a, self.pointer.down_at().0, x, double)
                    {
                        self.pointer
                            .set_last_tap((!double && !dragged).then_some((a, self.now_ms)));
                        return;
                    }
                    self.pointer.set_last_tap(Some((a, self.now_ms)));
                    // A press on the screen's way out is Back (F77).
                    let a = self.as_back_at(a, found.map(|f| f.1));
                    self.press_in(a);
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
        let Some(grab) = self.pointer.bar_held() else {
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

    /// Brings what drawing only looks up in step before a frame is drawn
    /// (`docs/SPEED.md` §3.3, `docs/COMPUTE.md` §4): each open vault's
    /// records read again when they changed, the codex32 worksheet's fit,
    /// and Learn's place and pages ([`Self::learn_place`]). Before every
    /// [`Self::render`].
    fn before_frame(&mut self) {
        let c = self.compute();
        for o in &self.vaults.open {
            self.vaults.warm(&c, o.id);
        }
        self.codex32_refit();
        self.learn_place();
    }

    /// Draws the frame. Costs a full draw of the screen; the answers the
    /// backup's cards share are worked out once and kept only until it
    /// returns, by whatever way it returns. Each drawing pass reads a
    /// [`drawing::View`]; what it noted for after it is written once it
    /// is drawn ([`drawing::After`]).
    fn render(&mut self) {
        self.drawing.clear();
        self.render_frame();
        self.drawing.clear();
    }

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn render_frame(&mut self) {
        let Some(mut canvas) = self.canvas.take() else {
            return;
        };
        // A change of screen or sheet fades in from the frame before it.
        self.fade_begin(&canvas);
        // The keyboard's place, while the last frame offered it: read
        // before its hits are taken for this one.
        let focused = self.pointer.focused();
        let mut hits = self.pointer.take_hits();
        hits.clear();
        // Offsets in whole pixels, whoever set them last, so a scrolled
        // page moves as one.
        let f = self.f.max(0.1);
        let whole = self.whole_offset();
        if let Some(offset) = self.scroll_slot() {
            *offset = whole(*offset);
        }
        let stretch = (self.scroll.stretch() * f).round() as i32;
        // The overlay scrollbar shows while the offset moves and fades
        // once it has rested a moment.
        let key = self.region_key();
        let slot = self.slot();
        let offset = self.scroll_slot().map_or(0.0, |o| *o);
        let page_offset = self.screen_slot().map_or(0.0, |o| *o);
        self.sheet_offset = self.page_slot().map_or(0.0, |o| *o);
        let (seen_key, seen_offset, moved_at) = self.scroll.bar_seen();
        let moved_at = if seen_key != (key, slot) {
            0
        } else if seen_offset != offset || stretch != 0 {
            self.now_ms.max(1)
        } else {
            moved_at
        };
        // Held, or with the pointer over it, it stays in full.
        let hot = self.pointer.bar_held().is_some()
            || self
                .pointer
                .hover()
                .is_some_and(|(x, y)| self.bar_at(x, y).is_some());
        let moved_at = if hot { self.now_ms.max(1) } else { moved_at };
        self.scroll.set_bar_seen((key, slot), offset, moved_at);
        let bar = motion::bar_alpha(moved_at, self.now_ms);
        // Drawn twice when a step card has just opened: the first time
        // finds out, the second draws it growing from closed.
        let mut follow_to = None;
        let (mut scrolled, mut caret, mut reach) = (Vec::new(), None, Vec::new());
        let mut tips = Vec::new();
        // A field newly focused brings the keyboard up, worked out before
        // drawing, which reads the keyboard as it now is; on a small panel
        // the page scrolls the field into view above it (`osk_follow`).
        self.osk_track();
        for _ in 0..3 {
            hits.clear();
            let disclosure = self
                .disclosed
                .as_ref()
                .filter(|d| d.key == key && d.moving)
                .map(|d| ui::Disclosure {
                    open: d.open,
                    closing: d.closing,
                    shown: motion::ease_open(motion::progress(
                        d.at,
                        self.now_ms,
                        motion::DISCLOSE_MS,
                    )),
                });
            let column = {
                let mut ui = ui::Ui::new(
                    &mut canvas,
                    self.f,
                    &mut hits,
                    self.pointer.pressed().map(|p| p.0),
                );
                ui.set_text_size(self.text_size());
                ui.edit = self.edit_now();
                ui.theme = self.theme();
                ui.hovered = self.pointer.hovered();
                ui.focused = focused;
                ui.tips.shown = self.pointer.tip_shown();
                ui.dimmed = self.demo_dimmed();
                ui.stretch = stretch;
                ui.bar = (bar > 0).then_some((bar, offset));
                ui.stretch_in_sheet = self.sheet.is_some_and(Sheet::scrolls_with_bar)
                    || (self.compact && self.sheet.is_some());
                ui.disclosure = disclosure;
                let frost_key = ((self.screen, self.sheet), self.theme());
                ui.frost = self.fade.take_frost(&frost_key);
                ui.guided_shown = self.guided_shown();
                ui.about_open = self.panel.about_on(self.screen, self.sheet.is_none());
                ui.offset = self.screen_slot().map_or(page_offset, |o| *o);
                ui.active = slot;
                ui.compact = self.compact;
                ui.caret_on = self.caret_on();
                let view = drawing::View::new(self);
                screens::draw(&view, &mut ui);
                ui.draw_tip();
                tips = std::mem::take(&mut ui.tips.anchors);
                self.apply_after(view.into_after());
                caret = ui.caret_drawn.then_some(ui.caret_on);
                self.osk_caret(ui.caret_foot);
                self.typing.set_fields(std::mem::take(&mut ui.fields));
                self.trail.set_link(self.screen, ui.back_link, ui.back_hit);
                scrolled = std::mem::take(&mut ui.scrolled);
                reach = ui.take_presses();
                self.pointer.set_cut(std::mem::take(&mut ui.cut));
                follow_to = follow_to.or(ui.follow_to);
                self.fade.keep_frost(frost_key, ui.frost.take());
                (ui.column, ui.focus_box, ui.focus_action)
            };
            if !(self.osk_follow(column.1, column.2, &scrolled) | self.disclose(key, column.0)) {
                break;
            }
        }
        if let Some(want) = follow_to {
            // A follow moves the page, whatever region the pointer was on.
            self.scroll.went_to(ui::Slot::Page);
            let now = self.scroll_slot().map_or(want, |o| *o);
            if self.reduce_motion() {
                self.with_region(|m, r| m.jump(r, want - now));
            } else {
                self.scroll.glide_to(want - now);
            }
            self.out.dirty = true;
            self.out.commands.push_back(Command::Draw);
        }
        let drew = self.region_key();
        self.scroll.drew(drew, scrolled);
        self.typing.set_caret_drawn(caret);
        self.vaults.set_drawn();
        self.later_drawn = true;
        self.pointer.set_hits(hits, reach);
        self.pointer.set_tip_anchors(tips);
        self.fade_end(&mut canvas);
        self.canvas = Some(canvas);
        // The frame moved things under a still pointer: the next frame
        // shows what it is over now.
        if self.find_hovered() {
            self.out.dirty = true;
            self.out.commands.push_back(Command::Draw);
        }
    }

    /// Whether the text caret shows now: it blinks, on for a half period
    /// and off for one, starting on at the last key or press. Steady with
    /// reduce motion.
    fn caret_on(&self) -> bool {
        self.typing.caret_on(self.now_ms, self.reduce_motion())
    }

    /// Where the Guided switch's pill shows: 0 on Steps only, 1 on
    /// Guided, between while it slides.
    fn guided_shown(&self) -> f32 {
        let to = if self.guided() { 1.0 } else { 0.0 };
        match self.guided_moving {
            Some((from, at)) => {
                let t = motion::ease_move(motion::progress(at, self.now_ms, motion::SWITCH_MS));
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
            Some(d) if d.key == key && d.open != open && self.settings.reduce_motion() => {
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
        self.typing.blink_from(self.now_ms);
        self.settle_regions();
        self.scroll.stop();
        self.pointer.set_scroll_at(None);
        if let Some(d) = self.disclosed.as_mut() {
            d.moving = false;
        }
        self.guided_moving = None;
        self.fresh.own_settle();
        let whole = self.whole_offset();
        let offset = self.scroll_slot().map_or(0.0, |o| whole(*o));
        self.scroll
            .set_bar_seen((self.region_key(), self.slot()), offset, 0);
        if self.out.toast.is_some() {
            self.out.toast_at = Some(self.now_ms.saturating_sub(motion::TOAST_MS));
        }
        // The next frame is the screen at rest, not on the way from the last.
        self.fade.restart();
        self.out.dirty = true;
    }

    /// Acts as if `action` had been pressed: for drivers that script the
    /// screens, such as the snapshot tool.
    pub fn press(&mut self, action: Action) {
        // An action may change the words typed: the status line's key is
        // worked out again, once typing pauses when the action is a
        // character from the on-screen keyboard.
        self.entry.state().forget_probe();
        if matches!(
            action,
            Action::EntryKey(_) | Action::EntryKeyBack | Action::EntryCandidate(_)
        ) && self.entry_typing_here()
        {
            self.inbox.set_typed_at(Some(self.now_ms));
        }
        self.note_secrets();
        self.note_here();
        self.press_in(action);
        // A script does not wait for frames: what a person's press leaves
        // for the tick after its frame is done now.
        self.later_now();
        self.backup_settle();
        self.after_input();
        self.out.dirty = true;
        self.out.commands.push_back(Command::Draw);
    }

    /// A person's press: done now, or, when its work stretches a key or
    /// makes pictures ([`slow`]), drawn first and done on the tick after
    /// (`docs/SPEED.md` §9). A press left waiting goes first.
    fn press_in(&mut self, action: Action) {
        // A slider moved again replaces the files it asked to be made.
        if !(matches!(action, Action::Slide(..))
            && matches!(self.later, Some(Later::PublicOut(..))))
        {
            self.later_now();
        }
        if slow(action) && !field_action(action) {
            self.later = Some(Later::Press(action));
            self.later_drawn = false;
            if let Some(text) = working_text(action) {
                self.working(text);
            }
            self.out.dirty = true;
            self.out.commands.push_back(Command::Draw);
            return;
        }
        let before = work::snapshot();
        self.act(action);
        // The rule `slow` keeps, held in debug builds and every test: a
        // press not left for the tick stretches no key and makes at most
        // one picture.
        let done = work::since(&before);
        debug_assert!(
            done.of(work::Work::Kdf) == 0 && done.of(work::Work::Picture) <= 1,
            "{action:?} stretched a key {} times and made {} pictures with no frame drawn first: \
             add it to `slow` (docs/SPEED.md §9)",
            done.of(work::Work::Kdf),
            done.of(work::Work::Picture),
        );
    }

    /// Says what a press left for the tick is doing, at once: the toast
    /// shows in full on the frame drawn before the work.
    fn working(&mut self, text: &str) {
        self.toast(text);
        self.out.toast_at = Some(self.now_ms.saturating_sub(motion::TOAST_MS));
    }

    /// Does the press waiting, once a frame has been drawn since it was
    /// pressed. Public files a slider asked for wait until it is let go.
    /// Returns whether it did.
    fn later_tick(&mut self) -> bool {
        let held = matches!(self.pointer.pressed(), Some((Action::Slide(..), _)));
        if !self.later_drawn
            || self.later.is_none()
            || (held && matches!(self.later, Some(Later::PublicOut(..))))
        {
            return false;
        }
        self.later_now();
        true
    }

    /// Does the press waiting now, whatever has been drawn.
    fn later_now(&mut self) {
        let Some(later) = self.later.take() else {
            return;
        };
        let toast = self.out.toast.as_ref().map(|(t, _)| t.clone());
        if let Later::Press(_) | Later::Key(_) = later {
            self.note_here();
        }
        match later {
            Later::Press(a) => self.act(a),
            Later::Key(k) => self.key(k),
            Later::PublicOut(w, what) => self.public_out(w, what),
        }
        if let Later::Press(_) | Later::Key(_) = later {
            self.backup_settle();
            self.after_input();
        }
        // The working toast goes once the work is done, unless the work
        // said something of its own.
        if toast.is_some()
            && self.out.toast.as_ref().map(|(t, _)| t.clone()) == toast
            && [EN.working_files, EN.working_node_key]
                .iter()
                .any(|w| toast.as_deref() == Some(*w))
        {
            self.out.toast = None;
        }
        self.out.dirty = true;
        self.out.commands.push_back(Command::Draw);
    }

    /// What every press does once it has acted, however it came (a tap,
    /// a key, the mouse's back button, a scripted press): a screen left
    /// drops what it held, a detour not taken ends, a spent round goes
    /// from its vault, and a sheet waiting comes up. Lookups, unless
    /// something is dropped.
    fn after_input(&mut self) {
        self.stick.note_visit(self.screen == Screen::Visit);
        // The screen the input left goes on the trail (F77).
        self.trail_note();
        // A vault being made or unlocked for another flow: the way back
        // ends when the person leaves for anything else, and a secret
        // kept for it goes, wiped. While on the way, the flow keeps what
        // it held.
        let detour = matches!(
            self.screen,
            Screen::CreateVault | Screen::Vaults | Screen::Unlock | Screen::BackupFlow
        );
        if !detour {
            self.vaults.set_back_to(None);
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
        // A vault's passphrase, typed or half typed, goes once nothing that
        // takes it is up, however it was left; an unlock under way keeps it
        // until it is done.
        let typing = matches!(
            self.screen,
            Screen::Unlock | Screen::BackupFlow | Screen::Family
        ) || self.sheet == Some(Sheet::Import);
        if !typing && self.vaults.working != Some(vault::Work::Unlock) {
            self.vaults.clear_unlock();
        }
        // Leaving New key drops what it held (Add randomness, opened from it, keeps it).
        if !matches!(self.screen, Screen::KeyGen | Screen::Randomness)
            && self.keygen.state().is_some()
        {
            self.keygen_drop();
        }
        if !bip85 {
            self.bip85.clear();
        }
        self.explore_left();
        if self.screen != Screen::Tools {
            self.tools.close();
        }
        if self.sheet != Some(Sheet::WordList) {
            self.wordlist.clear();
        }
        // Leaving the vanity screen stops a search and drops a find.
        if self.screen != Screen::Vanity {
            self.vanity.clear();
        }
        if !lightning && self.sheet != Some(Sheet::SecretOut) {
            self.lightning.clear();
        }
        if !silent && self.sheet != Some(Sheet::Qr) {
            self.silent.clear();
        }
        self.cameras_noise();
        self.keygen_device();
        self.release_held_stick();
        // A device still waiting comes back up once no other sheet is.
        self.input_sheet();
        self.device_sheet();
    }

    /// Whether the last frame drawn offers `action` to a press: for
    /// tests that a control is on screen.
    pub fn offers(&self, action: Action) -> bool {
        self.pointer.offers(action)
    }

    /// The time of the last tick, ms: what a test's next tick follows.
    pub fn now_ms(&self) -> u64 {
        self.now_ms
    }

    /// Where the last frame drawn offers `action`, in pixels: the middle
    /// of where it can be pressed, for tests that touch it.
    pub fn where_offered(&self, action: Action) -> Option<(u16, u16)> {
        let r = self.pointer.rect_of(action)?;
        Some(((r.x + r.w / 2) as u16, (r.y + r.h / 2) as u16))
    }

    /// The box the last frame drawn offers `action` in, in pixels
    /// (x, y, w, h): for a test that two controls do not overlap.
    pub fn hit_box(&self, action: Action) -> Option<(u16, u16, u16, u16)> {
        let r = self.pointer.rect_of(action)?;
        Some((r.x as u16, r.y as u16, r.w as u16, r.h as u16))
    }

    /// The width and height of the layout, in design units.
    pub fn size(&self) -> (f32, f32) {
        (self.w, self.h)
    }

    /// The keys loaded that no wallet uses, and the one of them the
    /// Wallets card offers a wallet from: the one picked, else the first.
    pub fn loose_keys(&self) -> (Vec<faraday_bip::keys::Fingerprint>, Option<[u8; 4]>) {
        let loose = self.session.loose_keys();
        let picked = self
            .wallets
            .loose_pick()
            .filter(|p| loose.iter().any(|f| f.0 == *p))
            .or_else(|| loose.first().map(|f| f.0));
        (loose, picked)
    }
}

pub use create::create_steps;

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

/// What a press leaves for the tick after the frame that shows it (`docs/SPEED.md` §9).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Later {
    /// A press whose work is [`slow`].
    Press(Action),
    /// A key whose work is slow (`Faraday::key_slow`, in `lightning`).
    Key(KeyIn),
    /// The backup's public files of kind `what` for wallet `w`, made again
    /// once a slider that changes them is let go.
    PublicOut(usize, u8),
}

use pointer::field_action;
use slow::{slow, working_text};

/// A file name without its extension.
pub(crate) fn stem(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 => name[..i].to_string(),
        _ => name.to_string(),
    }
}

impl App for Faraday {
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn event(&mut self, event: Event) {
        self.vault_shown_tidy();
        // Any event but a pointer's motion, a scroll, a hover, a tick, a
        // camera frame or a modifier may change the words typed on Add a
        // key or New key's Words card: the status line's key is worked
        // out again on the next frame that shows it, and frames during a
        // scroll or a hover work out nothing.
        if !matches!(
            event,
            Event::Touch {
                phase: TouchPhase::Move,
                ..
            } | Event::Scroll { .. }
                | Event::Wheel { .. }
                | Event::ScrollEnd { .. }
                | Event::Hover { .. }
                | Event::HoverEnd
                | Event::Tick { .. }
                | Event::CameraFrame { .. }
                | Event::Shift { .. }
                | Event::KeyUp(_)
        ) {
            self.entry.state().forget_probe();
            // A character typed into the words or the passphrase: the key
            // they make is worked out once typing pauses.
            if matches!(event, Event::Key(_)) && self.entry_typing_here() {
                self.inbox.set_typed_at(Some(self.now_ms));
            }
        }
        // Whether the event can complete a checklist item: pointer
        // moves, scrolls, hovers, camera frames and ticks cannot
        // (`docs/SPEED.md` §3.5).
        let settle = matches!(
            event,
            Event::Touch {
                phase: TouchPhase::Down | TouchPhase::Up,
                ..
            } | Event::Key(_)
                | Event::Back
                | Event::Forward
                | Event::Scanned { .. }
                | Event::CameraUnavailable
                | Event::CameraFailed(_)
                | Event::Clipboard { .. }
                | Event::ClipboardUnavailable { .. }
                | Event::Display(_)
        );
        if settle {
            self.note_here();
        }
        match event {
            Event::Display(d) => self.display(d),
            Event::Touch { x, y, phase } => {
                self.input_now();
                self.typing.blink_from(self.now_ms);
                // Any input puts the idle lock off; the press that does
                // it does nothing else.
                if self.sheet == Some(Sheet::IdleWarn) {
                    if phase == TouchPhase::Down {
                        self.sheet = None;
                        self.pointer.set_pressed(None);
                    }
                } else {
                    self.touch(x, y, phase);
                }
            }
            Event::Key(k) => {
                self.input_now();
                self.typing.blink_from(self.now_ms);
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.sheet = None;
                } else if self.key_slow(k) {
                    // Drawn first, done on the tick after (`docs/SPEED.md`
                    // §9).
                    self.later_now();
                    self.later = Some(Later::Key(k));
                    self.later_drawn = false;
                    self.working(EN.working_node_key);
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                    return;
                } else {
                    self.later_now();
                    self.key(k);
                }
                self.pointer.tip_focus(self.now_ms);
            }
            Event::Shift { held } => {
                self.typing.set_shift(held);
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
                if self.pointer.scroll_at().is_none() {
                    self.point_scroll(i32::from(x), i32::from(y), f32::from(dy));
                }
                // Pixels the content moves up, as the shell API states
                // them, in this layout's units.
                self.pan(f32::from(dy) / self.f.max(0.1));
                self.pointer.set_scroll_at(Some(self.now_ms));
            }
            Event::Wheel { x, y, dy } => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.sheet = None;
                }
                self.point_region(i32::from(x), i32::from(y));
                // The shells send 48 pixels a notch; the notch's distance
                // is this layout's, in units, so it covers the same amount
                // of page on every screen.
                self.wheel(f32::from(dy) / motion::SHELL_NOTCH_PX * motion::WHEEL_NOTCH);
            }
            Event::ScrollEnd { .. } => {
                self.pointer.set_scroll_at(None);
                self.scroll.release(self.f);
            }
            // A frame only when what is under the pointer changes.
            Event::Hover { x, y } => {
                self.input_now();
                let was = self
                    .pointer
                    .hover()
                    .is_some_and(|(x, y)| self.bar_at(x, y).is_some());
                self.pointer.set_hover(Some((i32::from(x), i32::from(y))));
                let now = self
                    .pointer
                    .hover()
                    .is_some_and(|(x, y)| self.bar_at(x, y).is_some());
                let tip = self.pointer.tip_hover(self.now_ms) | self.own_pointer(x, y);
                // Coming over the scrollbar shows it; a move taken draws.
                if !self.find_hovered() && was == now && !tip {
                    return;
                }
            }
            Event::HoverEnd => {
                self.pointer.set_hover(None);
                let tip = self.pointer.tip_hover(self.now_ms) | self.own_flush();
                if !self.find_hovered() && !tip {
                    return;
                }
            }
            Event::Tick { now_ms } => {
                // A fade under way runs to its end whatever the gaps
                // say. Its clock starts now if the frame that showed the
                // change was the last one, a frame back, so this frame
                // is the first step of it rather than the old screen
                // again.
                self.fade.tick_clock(now_ms);
                // The scrollbar was showing at the last tick: this one
                // draws its next step of fading, the last one included.
                let fading = motion::bar_alpha(self.scroll.bar_moved_at(), self.now_ms) > 0;
                self.now_ms = now_ms;
                // Asked for on every tick while something moves, so the
                // shell keeps drawing at its frame rate. A card opening or
                // a toast coming or going starts on its first tick.
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
                if self.out.toast.is_some() && self.out.toast_at.is_none() {
                    self.out.toast_at = Some(now_ms);
                }
                let fading = fading
                    || self.typing.caret_changed(self.caret_on())
                    || self.disclosed.is_some_and(|d| d.moving)
                    || self.guided_moving.is_some()
                    || self.out.toast.as_ref().is_some_and(|(_, until)| {
                        self.out
                            .toast_at
                            .is_some_and(|at| now_ms < at + motion::TOAST_MS + 16)
                            || now_ms + motion::TOAST_MS + 16 > *until
                    });
                // Scrolling that stopped with no end said (a shell that
                // never sends one) ends here, so a stretch springs back.
                if self
                    .pointer
                    .scroll_at()
                    .is_some_and(|at| now_ms.saturating_sub(at) > motion::SCROLL_QUIET_MS)
                {
                    self.pointer.set_scroll_at(None);
                    self.scroll.release(self.f);
                }
                let tip = self.pointer.tip_tick(now_ms) | self.keygen.dice_tick(now_ms);
                if self.own_tick(now_ms) | self.motion_tick(now_ms) || fading || tip {
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                }
                // A fade under way asks for its next step; unless
                // something else asks for a frame too, the step blends
                // the two kept frames (`Faraday::fade_step`).
                if self.fade.tick_due() {
                    self.out.commands.push_back(Command::Draw);
                }
                if self.idle_tick(now_ms) {
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                    return;
                }
                // Each runs whatever the others return.
                if self.vault_tick(now_ms)
                    | self.later_tick()
                    | self.keygen_tick(now_ms)
                    | self.seed_tick()
                    | self.typed_tick(now_ms)
                    | self.explore_tick(now_ms)
                    | self.chart_side_tick(now_ms)
                    | self.key_path_sheet_open()
                {
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                }
                // A live clock or a TOTP code on screen moves on by the
                // second, with nothing else moving.
                if self.shows_clock() {
                    let now = self.clock_now();
                    if now != self.clock_drawn {
                        self.clock_drawn = now;
                        self.out.dirty = true;
                        self.out.commands.push_back(Command::Draw);
                    }
                }
                self.vanity_tick(now_ms);
                if self.scan_tick(now_ms) {
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                }
                self.zero_tick();
                if self.qr.tick(now_ms, self.settings.qr_frame_ms()) {
                    self.out.dirty = true;
                    self.out.commands.push_back(Command::Draw);
                }
                if let Some((_, until)) = &self.out.toast
                    && now_ms > *until
                {
                    self.out.toast = None;
                } else {
                    return;
                }
            }
            Event::EntropySource(name) => return self.entropy_source_named(name),
            Event::Entropy(e) => return self.entropy_arrived(e),
            Event::CameraFrame { luma, .. } if self.noise_camera_on() => self.noise_frame(luma),
            Event::CameraFrame {
                width,
                height,
                luma,
                chroma,
            } => match self.camera.scan_mut() {
                Some(s) => {
                    // The frame replaced is wiped as it drops.
                    s.frame = Some((width, height, zeroize::Zeroizing::new(luma)));
                    s.chroma = chroma.map(zeroize::Zeroizing::new);
                    s.frames = s.frames.wrapping_add(1);
                    s.waiting_since = None;
                    if s.no_picture == Some(NoPicture::Silent) {
                        s.no_picture = None;
                    }
                }
                None => return,
            },
            Event::Scanned { bytes } => {
                self.scanned(bytes);
                self.release_held_stick();
            }
            Event::CameraUnavailable => self.camera_failed(NoPicture::Unavailable),
            Event::CameraFailed(why) => self.camera_failed(match why {
                faraday_shell_api::CameraFailure::Refused => NoPicture::Refused,
                faraday_shell_api::CameraFailure::InUse => NoPicture::InUse,
                faraday_shell_api::CameraFailure::NotFound => NoPicture::NotFound,
            }),
            Event::Clipboard { text, .. } => self.pasted(zeroize::Zeroizing::new(text)),
            Event::ClipboardUnavailable { .. } => self.pasted(String::new().into()),
            Event::RotationOffered { native } => self.rotation_offered_by(native),
            _ => return,
        }
        if settle {
            self.backup_settle();
            self.after_input();
        }
        self.out.dirty = true;
        self.out.commands.push_back(Command::Draw);
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.note_secrets();
        self.out.commands.pop_front()
    }

    fn frame(&mut self) -> Frame<'_> {
        if self.out.dirty {
            self.out.dirty = false;
            self.before_frame();
            self.render();
        } else if self.fade.is_due() {
            self.fade_step();
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
        let c = self.compute();
        self.last_display = Some(d);
        // The self-test runs once, at start, before any input: a display
        // comes before anything can be pressed.
        if self.selftest.is_none() {
            self.selftest = Some(faraday_selftest::run(&c));
        }
        self.fade.display_changed();
        // Laid out at the screen's own density, never scaled to fit a page
        // (`docs/LAYOUT.md` §7, F101); the person's Display scale
        // multiplies it.
        let f = self.pixels_per_unit_of(&d);
        // One-column pages but beside the expanded rail (LAYOUT §3); held
        // inside the Starter, under the wide layout's 1280 dp.
        self.compact = self.one_column();
        let scaled = DisplayInfo {
            dpi: (160.0 * f).round().max(1.0) as u16,
            ..d
        };
        self.f = f;
        self.w = f32::from(d.width) / f;
        self.h = f32::from(d.height) / f;
        let mut canvas = Canvas::new(&scaled);
        // The canvas notes its pixel work, a blur or a whole copy, with
        // this crate's counter (`docs/COMPUTE.md` §2.2).
        canvas.set_counter(work::core_count);
        self.canvas = Some(canvas);
        self.fresh_ask();
    }
}

/// Whether a pass over this transaction may open a MuSig2 round, which
/// draws secret nonces: any such input is one, since what the pass does
/// with it is decided only as it signs.
pub(crate) fn spend_opens_round(inspection: &faraday_psbt::Inspection) -> bool {
    inspection.inputs.iter().any(|i| i.musig.is_some())
}
