//! Faraday's app: the screens and the state of each area (the vault, the
//! backup flows, the Wallets tab's session, the Inbox and the Outbox), every
//! Tools flow and the stick rule. The backup plan's logic (`faraday-plan`), the
//! readers of formats it takes in (`faraday-core-wallet`), the PDF writer, the
//! printed sheets, EDID and build ID (`faraday-paper`) and Faraday Demo's host
//! (`faraday-host`) are crates of their own. How the
//! code is laid out is `docs/STRUCTURE-PASS.md`; the screens are
//! `docs/DESIGN.md`; the areas are `docs/VAULT.md`, `docs/STARTER.md` and
//! `docs/KEYS.md`.
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
//! here (`tools/lint-layout.py` fails when one is not):
//!
//! - The shell's protocol and the app's types: `storage_api` (sticks and
//!   storage events), `version`, `action` (the `Action` enum), `qr_view`
//!   (the QR view and the camera's scan state), `receipt` and `state` (the
//!   state of Entry, Message and the stick visit). Each other area's state
//!   is in its own `mod.rs` or file: `DecodeState` in `decode`,
//!   `SpendState` in `wallet`, `BackupState` in `wallet_backup`,
//!   `CreateState` in `create`, `RestoreState` in `restore`. `lib.rs`
//!   re-exports them all, so a caller names `faraday_core::Action`.
//! - State and actions: `work` (costly work counted), `compute` (where
//!   event code gets its token), `flow`, `inputs`, `edit` (the caret and
//!   the selection), `motion` (scrolling), `moved` (the moved frame), `out` (what the core
//!   hands the shell: commands, storage commands, the toast), `pointer` (the
//!   hits, the press, the drag, the hover), `idle` (the idle lock's timers), `item` (an Inbox or Outbox file), `worker` (the one thread costly work runs on), `fade` (a
//!   change fading in), `menu` (the ⋮ menus), `nav` (the navigation by
//!   window class, with `nav/screen.rs`), `file_menu`, `in_progress`,
//!   `compact` (the small panel), `panes` (list and detail), `medium`
//!   (what the removable medium is called), `memory` (what is kept across
//!   a lock), `copy_check` (a copy by hand checked), `check` (Check, the list of what to hold up against a hardware wallet, with `check/screen.rs`), `set_time`, `stick`
//!   (`pills.rs`, `events.rs`, `folders.rs`, `settings.rs`, `wipe.rs`, `visit_taken.rs`), `trail` (where
//!   Back lands: the screens left, as they were seen), and `ui` (drawing
//!   in design units).
//! - Screens and drawing: `screens` (the frame, the sidebar, the sheets and
//!   the screens with no area folder), `drawing`, each area's `screen.rs`
//!   (`vault/screen.rs`, `keygen/screen.rs`), `glance` (a wallet at a
//!   glance), `address_find` (what Explore's Find an address and the address check's Look further share), `scan_key`, `scan_read`
//!   (what a code read is), `text_check` (the secrets a text holds, refused by Transfer's Send text and by Compare with the wallet app's), `picture` (a QR code to keep), `demo` (what Practice refuses),
//!   `devices` (what else captures the screen or the keys).
//! - Flows around a key, seed, words or passphrase: `keygen`, `codex32` (with `codex32/screen.rs`, its checksum worksheet, `codex32/worksheet_screen.rs`, and its dice and the booklet's dice sheet, `codex32/dice_screen.rs`, and the entry's input panel, `codex32/input_panel.rs`; New key's dice in the input panel, `keygen/dice_panel.rs`), `seeds`
//!   (with `seeds/read.rs`), `wallet` (with `wallet/nonce.rs`, `wallet/account.rs`), `inbox`, `create`, `create_from`,
//!   `bip85`, `forms`, `fresh`, `gpg`, `secure_boot`, `lightning`, `silent`, `vanity`, `grind`, `quiz`, `secrets`, `word_entry`, `vault` (with `vault/load.rs` and `vault/out.rs`), `kef_form` (the form that makes a Krux code, `kef_form/screen.rs`), `dice_entry` (the dice entry Create a vault uses, `dice_entry/screen.rs`), `chart_edit`, `boot_import`, `backup_flow/wallet.rs` and `codex32_correct` (§8.4's correction, with its `screen.rs`) and `core_wallet` (the tool and screen that read a Bitcoin Core wallet through `faraday-core-wallet`, F229). They hold
//!   handles and public results; key use is `faraday-keys` (F70,
//!   `docs/KEYS.md`), and `tools/lint-secrets.sh` rule 4 holds it.
//! - Wallets and PSBTs: `plan`, `decode`, `spend_backup`, `explore`, `catalog`, `register`, `review`, `signing`, `cstep` (Make a new wallet's cards, numbered).
//! - Backups and the backup flow: `backup_flow`, `backups`, `board`,
//!   `held` (the backup before spending, and signing), `restore`, `wallet_backup`, `public_files`,
//!   `codex32_backup` (its
//!   codex32 item and the set's actions, with its `screen.rs`).
//! - Vaults: `codes` (TOTP codes); `vault` is listed with the flows above.
//! - Sticks, boot and files: `transfer` (the desktop as the
//!   device's QR link), `names` (the Starter's install names),
//!   `secure_boot/state.rs`.
//! - Faraday Demo's host is the `faraday-host` crate (`docs/DESKTOP-HOST.md`); the app gives it the
//!   fields and methods marked "Used by faraday-host."
//! - Support: `slow` (the presses whose work waits for a frame), `visible` (a message's hidden characters drawn as marks, kept while it is the same), `wordlist`, `guide`, `modes` (F81, with `modes/screen.rs`), `samples` (with `samples/screen.rs`, and `samples/put.rs`, Use a sample, whose buttons Faraday Demo's strip shows), `tools`, `testkit`, `leaks` (the tours' leak check's planted forms, `testkit` only).
//!
//! `Faraday::act_inner` is the dispatch for presses: it runs the checks that
//! come before any action, then sends each `Action` to its area's
//! `<area>_act` (`create_act` in `create`, `restore_act` in `restore`,
//! `backup_act` in `wallet_backup`, `wallet_act` in `wallet`, `visit_act` in
//! `visit_taken` and the like). Only navigation, Back, the settings and the
//! the guide sheet are handled in `act_inner` itself.
//!
//! Every `Screen` variant and the file that draws it. An area draws in its
//! own `screen.rs`; a screen with no area folder is in `screens/`, whose
//! `mod.rs` holds the frame and the dispatch, `sidebar.rs` the sidebar, `sheets.rs` the sheets,
//! `guide_pane.rs` the guide's supporting pane and `parts.rs` what two or more screens share.
//!
//! | `Screen` | Drawn in |
//! |---|---|
//! | `Home` | `screens/home.rs` |
//! | `WalletsStart` | `screens/wallets_start.rs` |
//! | `Wallets` | `screens/wallets.rs` |
//! | `Spend` | `screens/spend.rs` |
//! | `Files` | `screens/files.rs` |
//! | `Entry` | `screens/entry.rs` |
//! | `Backup` | `screens/backup.rs` |
//! | `Message` | `screens/message.rs` |
//! | `Check` | `check/screen.rs` |
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
//! | `Convert` | `convert/screen.rs` |
//! | `KefTool` | `kef_tool/screen.rs` |
//! | `QrTool` | `qr_tool/screen.rs` |
//! | `OtsCheck` | `ots_check/screen.rs` |
//! | `SeedCheck` | `seed_check/screen.rs` |
//! | `MultisigCheck` | `multisig_check/screen.rs` |
//! | `CoreWallet` | `core_wallet/screen.rs` |
//! | `Silent` | `silent/screen.rs` |
//! | `Explore` | `explore/screen.rs` |
//! | `Lightning` | `lightning/screen.rs` |
//! | `Tools` | `tools/screen.rs` |
//! | `Unlock` | `vault/screen.rs` |
//! | `VaultContents` | `vault/screen.rs` |
//! | `Codes` | `codes/screen.rs` |
//! | `SpendBackup` | `spend_backup/screen.rs` |
//! | `Vanity` | `vanity/screen.rs` |
//! | `Decode` | `decode/screen.rs` |
//! | `Invoice` | `invoice/screen.rs` |
//! | `CheckAddress` | `address_check/screen.rs` |
//! | `NonceCheck` | `screens/nonce_check.rs` |
//! | `SignerCheck` | `decode/check_screen.rs` |
//! | `Catalog` | `screens/catalog.rs` |
//! | `Transfer` | `transfer/screen.rs` |
//! | `BackupFlow` | `backup_flow/screen.rs` |

// The shell's protocol and the app's types: storage, version, actions, QR view, receipt, state.
mod action;
pub mod address_check;
mod address_find;
mod qr_view;
mod receipt;
mod state;
mod storage_api;
mod version;

// State and actions: the app's state, its events and what a press does.
mod compact;
mod compute;
pub mod copy_check;
pub mod edit;
mod fade;
pub mod file_menu;
pub mod flow;
mod idle;
pub mod in_progress;
pub mod inputs;
mod item;
pub mod medium;
pub mod memory;
mod menu;
pub mod motion;
mod moved;
pub mod nav;
mod out;
mod panes;
mod pointer;
mod public_files;
pub mod set_time;
pub mod stick;
mod trail;
pub mod ui;

pub mod work;
pub mod worker;

// Screens and drawing: what is drawn from the state (each area's `screen.rs` is in its folder).
pub mod demo;
pub mod devices;
pub mod dice_entry;
mod drawing;
pub mod glance;
pub mod picture;
mod scan_key;
pub mod scan_read;
mod screens;
pub(crate) mod text_check;
mod visible;

// Flows around a key, seed, words or passphrase; key use is faraday-keys (F70).
pub mod bip85;
pub mod boot_import;
pub mod chart_edit;
pub mod codex32;
pub mod codex32_correct;
pub mod convert;
pub mod core_wallet;
pub mod create;
mod create_from;
pub mod forms;
pub mod fresh;
pub mod gpg;
pub mod grind;
pub mod inbox;
pub mod invoice;
pub mod kef_form;
pub mod kef_tool;
pub mod keygen;
pub mod lightning;
pub mod multisig_check;
pub mod ots_check;
pub mod qr_tool;
pub mod quiz;
mod review;
pub mod secrets;
pub mod secure_boot;
pub mod seed_check;
pub mod seeds;
mod signing;
pub mod silent;
pub mod vanity;
pub mod vault;
pub mod wallet;
pub mod word_entry;

// Wallets and PSBTs.
pub mod catalog;
pub mod check;
pub mod decode;
pub mod explore;
pub mod plan;
mod register;
mod slow;
pub mod spend_backup;

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
#[cfg(feature = "demo")]
pub mod transfer;

// Support: word lists, the guide pages, the tools page, test data.
pub mod guide;
#[cfg(feature = "testkit")]
pub mod leaks;
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
pub use item::Item;
pub use medium::Medium;
pub use modes::{DESKTOP_MODES, Mode, STICK_MODES};
pub use qr_view::{
    Ask, Code, NoPicture, QR_PARTS, QR_SPEEDS, QrFormat, QrSource, QrView, SEEN_FRAMES,
    ScanPurpose, ScanState, Seen, TOO_FINE_TENTHS,
};
pub(crate) use qr_view::{UrFed, send_error, transfer_name, ur_feed, ur_note};
pub(crate) use receipt::sha256_of;
pub use receipt::{Receipt, ReceiptFile};
pub use restore::RestoreState;
pub use screens::qr_code;
pub use secure_boot::state::SecureBootAtStart;
pub use state::{EntryState, MessageState};
pub use stick::folders::{BOOT_FOLDER, FIRST_BACKUP, backup_folder};
pub use stick::pills::{Pill, SticksWanted};
pub use stick::visit_taken::VisitState;
pub use storage_api::{
    BootMedium, Files, Kept, OnTaken, StickInfo, StickMake, StorageCommand, StorageEvent,
    files_from, kept_from,
};
pub use version::{
    BUILD, Program, VERSION, is_release, parse_version_output, version_label, version_line,
};
pub use wallet::SpendState;
pub use wallet_backup::BackupState;

use crate::strings::EN;
use faraday_intake::seedqr_words;
#[cfg(feature = "demo")]
use faraday_intake::{arrival, seed_in_code};

use faraday_crypto::work::Compute;
use faraday_draw::Canvas;
use faraday_shell_api::{App, Command, DisplayInfo, Event, Frame, Key as KeyIn, TouchPhase};
use wallet::{FileKind, Session, fp_text};

/// Minutes without input before the session locks, at first (F58).
pub const IDLE_LOCK_MIN: u16 = 30;
/// Minutes without input before the machine powers off, at first,
/// counted from the last input across the lock (F58).
pub(crate) const IDLE_OFF_MIN: u16 = 60;
/// How long before an idle lock its warning comes up, at most: half the
/// lock time when that is shorter.
pub const IDLE_WARN_MS: u64 = 5 * 60_000;

/// The exit code that asks the app loop for a fresh process (`docs/SECURITY-MODEL.md`).
pub const RESTART_CODE: u8 = 75;

/// The exit code that asks init to restart the computer instead of
/// powering it off (`docs/FLOWS.md`, Restart; F42). 75 stays the lock's.
pub const RESTART_COMPUTER_CODE: u8 = 76;

/// The screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Home.
    Home,
    /// Check: what to hold up against a hardware wallet, each row opening its tool.
    Check,
    /// The Wallets tab's first page: every flow, by what the person has.
    WalletsStart,
    /// The Wallets tab: the wallets, the wallet card, and every flow (`docs/WALLETS.md`).
    Wallets,
    /// Signing a transaction.
    Spend,
    /// Inbox and Outbox, and the stick pane.
    Files,
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
    /// Every wallet known and where its backup is.
    Backups,
    /// Making a vault.
    CreateVault,
    /// Making a new key.
    KeyGen,
    /// BIP-85: child seeds, keys and passwords from a loaded key.
    Bip85,
    /// A loaded key written in another standard, the same wallet.
    Convert,
    /// Tools' Krux code: a text or a wallet's descriptor as an encrypted code.
    KefTool,
    /// Tools' QR vault: a note or a key sealed as one QR code.
    QrTool,
    /// Tools' Check a timestamp: an OpenTimestamps proof against a block header.
    OtsCheck,
    /// Tools' My seed makes my wallet: a seed's xpub and addresses.
    SeedCheck,
    /// Check a multisig setup: its quorum, checksum and cosigners.
    MultisigCheck,
    /// Tools' Read a Bitcoin Core wallet: a `wallet.dat` read and checked.
    CoreWallet,
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
    SpendBackup,
    /// A vanity address for a loaded key.
    Vanity,
    /// A transaction taken apart.
    Decode,
    /// A Lightning invoice taken apart.
    Invoice,
    /// Check an address: typed, pasted or scanned, searched in the loaded wallets.
    CheckAddress,
    /// A signed transaction brought in, checked rather than signed.
    NonceCheck,
    /// Check a signer: the PSBT sent and the one returned, compared.
    SignerCheck,
    /// Tools: every flow on one page, with its standards.
    Catalog,
    /// The online app's QR link (`transfer`): files and text out, camera in. Online only.
    #[cfg(feature = "demo")]
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

/// A file named as Faraday names a seed it writes out: its words, its SeedQR, a BIP-85 child.
fn seed_by_name(name: &str) -> bool {
    let lower = faraday_text::Text::ascii_lowercase(name);
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
    let lower = faraday_text::Text::ascii_lowercase(name);
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "psbt" => Some(EN.stick_kind_psbt),
        "txt" | "json" | "descriptor" => Some(EN.stick_kind_text),
        "txn" => Some(EN.stick_kind_transaction),
        "faraday-sheet" => Some(EN.stick_kind_print_sheet),
        "kdbx" => Some(EN.stick_kind_vault),
        "efi" => Some(EN.stick_kind_efi_image),
        "png" => Some(EN.stick_kind_qr_codes),
        _ => Some(EN.stick_kind_file),
    }
}

/// A sheet over the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sheet {
    /// A stick arrived while a secret is held.
    Lock,
    /// Lock asked for while a vault is open: what is sealed and not saved, or lost.
    LockAsk,
    /// Remove every key, while a key is kept nowhere else: the list, and
    /// the hold.
    RemoveKeys,
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
    /// The guide pages for the screen on show.
    Guide,
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
    /// A press that loads a key, a stick attached, carries on once it is pulled ([`Faraday::pull`]).
    Pull,
    /// A node or line of the wallet's chart: what it is, where, its actions (`docs/WALLETS.md` §12).
    Chart,
    /// The backup board's list, opened by **Add** in the narrow column (§17.12 item 3).
    BoardList,
    /// A drop let a place see the balance it could not before (`docs/BACKUP-FLOW.md` §17.5): **Keep it** or **Take it off**.
    BoardSees,
    /// Partial descriptions added (`docs/BACKUP-FLOW.md` §17.19 item 4): **Take it off every place** or **Keep it**.
    BoardPartials,
    /// A chip's ⋮ menu on the backup board (`docs/BACKUP-FLOW.md`
    /// §17.20): a popup under the ⋮ on `wide`, a sheet on the small panel.
    BoardMenu,
    /// A stick's ⋮ menu at Bring in the sticks (`backup_flow/figures.rs`).
    FigMenu,
    /// **Check it later**, asked first (`backup_flow/figures.rs`).
    LaterAsk,
    /// A checked stick's sheet (`backup_flow/stick_sheet.rs`).
    StickSheet,
    /// A seed the camera read, asking how it loads: with no passphrase or with one (`seed_read`).
    SeedRead,
    /// A Krux encrypted code: its passphrase, the bar while it opens, what came out (`seeds/kef`).
    KefRead,
    /// **Sweep a private key**: a WIF's addresses, its PSBT reviewed and signed (`seeds::sweep`).
    Sweep,
    /// Delete files: chosen files, free space or the files partition, asked first (`stick_wipe`).
    Delete,
    /// Zeroing under way: how far, and Stop.
    Zeroing,
    /// A stick visit's write would go over files on the stick: Replace, Keep both or Cancel (`visit_taken`).
    Taken,
    /// A vault's write refused for its name, a file on the stick having it and nothing proving a copy: Hold to overwrite, Save beside or Cancel (`visit_taken`).
    NameTaken,
    /// A picture on a stick whose code reads as a seed or as text: **Load as a key** or **Leave as a file** (`boot_import/text_ask`).
    TextSeed,
    /// What is attached that can capture the screen or the keys, each named
    /// with what it can do, and before a secret the reminders (`devices`).
    Devices,
    /// A wallet of several keys, or one a key not of its own can spend alone, waiting to be added: its rows, Cancel and a hold (F254).
    WalletConfirm,
    /// A file For the stick no stick holds, asked about before it is deleted: what is lost, a hold and Cancel (`visit_taken`).
    OutboxDelete,
    /// **Which account to load?**: a Coldcard export of a single-sig and a silent payments account.
    Account,
    /// The wallet a spend's PSBT defines, to register with a hold (F135).
    Register,
    /// A vault record's Save to a stick: files to tick (`docs/BACKUP-FLOW.md` §17.7).
    VaultSave,
    /// Cancel pressed in a flow whose ending loses something: what is
    /// lost, **Cancel the flow** and **Keep going** (F51).
    CancelFlow(in_progress::Going),
    /// The same in a key flow opened from Make a new wallet (`docs/FLOWS.md`, Cancel).
    CancelKey,
    /// A file row's ⋮ menu (`docs/FLOWS.md` decision 61): popup on `wide`, else a sheet.
    FileMenu,
    /// **Rename {name}**: a file For the stick, or one on the stick visited.
    Rename,
    /// A file's Details: name, kind, size and SHA-256.
    Details,
    /// **Load from {vault}**, after an unlock: what to load, ticked (F62).
    UnlockLoad,
    /// **Open this copy?**: an unlock held at an older copy (`vault::OpenHold`).
    OlderCopy,
    /// A passphrase or share the vault holds one of: **Continue** or **Cancel**.
    SaveAlso,
    /// **Change passphrase** on the open vault.
    VaultPassphrase,
    /// **Memory** of the open vault.
    VaultMemory,
    /// **Copy to another vault**: the open vaults an item can go to.
    CopyTo,
    /// **Save into {vault}**: what a seed keeps there.
    SaveKeeps,
    /// **Remove the codex32 set** or **the BIP-85 origin** of a seed in an open vault.
    Trim,
    /// **Save as a QR vault** on an item that does not fit.
    QrNoFit,
    /// **Choose a key file**: the files in Files that are not vaults.
    KeyFile,
    /// **Record it as a child seed?**: BIP-85 child words held (F166).
    ChildRecord,
    /// Tools' **GPG key** with no vault open: **Make a key** or **Restore from a paperkey**.
    GpgTile,
    /// Check's **A backup in another form**: the forms to choose from.
    CheckBackups,
    /// Check a signer's **Load the seed**: the ways to load the seed of
    /// the key that signed (`decode/check_screen.rs`).
    SignerSeed,
    /// **Options** of the Krux code tool's form.
    KefOptions,
    /// **A BIP-85 child seed**: a wallet made over one recorded (F166).
    ChildNotice,
    /// **Make a wallet** at an Explore path made one: its descriptor and first addresses (`explore/made.rs`).
    MadeWallet,
    /// **Seeds with no passphrase stored**, from a wallet short of a key: one to
    /// load with a passphrase typed now, checked against it (`vault_load`, F62).
    PassphraseSeeds,
    /// **Sign with {key}**: the files, the time the signatures carry, the hold that signs (F63).
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
    /// Check an address's From a file: the menu of text files in Files (`address_check/screen.rs`).
    AddressFiles,
    /// **Published words**: a key on Mainnet whose words are published, **Remove** or **Keep for testing**.
    Published,
    /// **Restore a GPG key**: the public keys in Files.
    GpgRestore,
}

pub(crate) use picture::{picture_names, picture_payloads};

/// Whether an Outbox file can go as a QR transfer: what a wallet reads goes as itself, anything
/// else in the file envelope (up to 256 KiB). A vault goes by stick only, whatever its size.
pub(crate) fn qr_fits(item: &Item) -> bool {
    match item.kind {
        FileKind::Vault => false,
        FileKind::Psbt | FileKind::Wallet | FileKind::Key | FileKind::Message | FileKind::Share => {
            true
        }
        _ => !item.bytes.is_empty() && item.bytes.len() <= faraday_qr::envelope::MAX_FILE,
    }
}

/// An account key of a loaded wallet held here, as [`Faraday::wallet_key`] finds it.
pub struct WalletKey {
    /// The master fingerprint, as text.
    pub fp: faraday_text::Text,
    /// `[fingerprint/path]xpub`, as the wallet's descriptor writes it.
    pub text: faraday_text::Text,
    /// What kind of wallet it is a key of, by name.
    pub kind: faraday_text::Text,
    /// The kind, when the key is at Create's own account for it.
    pub standard: Option<create::NewKind>,
    /// Whether this key's own origin path is at BIP-87's purpose (87'),
    /// as against BIP-48's: what its account code and key record are
    /// made at.
    pub bip87: bool,
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

/// The most vaults a backup is set up with at once: what a plan holds,
/// one checklist item number each.
const MAX_VAULTS: usize = faraday_plan::MAX_VAULTS;

/// The most sticks a backup is set up with at once; a count past it is
/// not a count a person means.
const MAX_STICKS: usize = faraday_plan::MAX_STICKS;

/// What [`Action::About`] names for a page's walk-through rather than a step's.
pub(crate) const ABOUT_PAGE: u8 = u8::MAX;

/// The create cards, in order.
pub mod cstep;

/// What Restore says of a seed that is not one of the description's keys.
pub const NOT_A_KEY: &str = EN.app_not_wallet_key;

/// The restore cards, in order.
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

/// The cards of the backup's plan, in order (`docs/BACKUP-FLOW.md` §15.1).
pub mod qstep {
    /// The watch-only software and the files' form.
    pub const SOFTWARE: u8 = 4;
    /// The plan: the map, approved with Make the checklist.
    pub const PLAN: u8 = 6;
    /// The board: the places, what each keeps, and the list of what the
    /// backup can keep (`docs/BACKUP-FLOW.md` §17).
    pub const BOARD: u8 = 8;
    /// The most places a plan has.
    pub(crate) const MAX_PLACES: usize = 240;
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

pub use plan::question as qrow;

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
pub type Writing = (
    faraday_text::Text,
    faraday_text::Text,
    Vec<(faraday_text::Text, [u8; 32])>,
    bool,
);

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
    /// and time zone (`stick/settings.rs`).
    settings: stick::settings::Settings,
    /// Setting the time by hand (`docs/TOTP.md` §3).
    set_time: set_time::SetTimeArea,
    /// The idle lock's timers and what its warning shows.
    idle: idle::Idle,
    /// A secret has been held or typed in this process. Only a fresh
    /// process is clean (`docs/SECURITY-MODEL.md`).
    tainted: bool,
    /// New input devices waiting to be believed, and those kept out
    /// (`docs/SECURITY-MODEL.md`).
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
    /// each wallet's backup: kept across a
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
    visit: stick::visit_taken::VisitArea,
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
    spend_backup: spend_backup::SpendBackupArea,
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
    /// The guide sheet and the steps ticked on its pages.
    guide: guide::GuideArea,
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
    #[cfg(feature = "demo")]
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
    /// The worker, the job that runs on it and whether a frame has been
    /// drawn since a press was left to wait.
    worker: worker::Worker,
    /// The step column's open card, and one opening or closing.
    disclosed: Option<Disclosed>,
    /// A change of screen or sheet fading in, how fast this machine has
    /// lately shown a fade's steps, the frosted page under the open
    /// sheet, and the screen the last frame drew (`fade.rs`).
    fade: fade::Fade,
    /// What the start-up self-test found (`faraday_selftest`): run when the
    /// display first arrives, before any input. A failure blocks the app
    /// on a screen naming the check, with Exit its only control.
    selftest: Option<faraday_selftest::Outcome>,
    /// Running on an online machine (the desktop app); sheets become PDFs.
    pub online: bool,
    /// A seed typed here is on an online computer: the seed line is drawn.
    pub seed_line: bool,
    /// Held by another app that shows only Transfer and the guide sheet
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
    /// What the removable medium is called and drawn as: a stick, or a
    /// card or stick on the Pi. The shell sets it at start.
    pub medium: Medium,
    /// Whether the machine started with Secure Boot on, as the stick shell
    /// read it once at start. Unknown everywhere else.
    pub secure_boot: SecureBootAtStart,
    /// The computer's maker (`sys_vendor`), as the stick shell read it.
    pub maker: faraday_text::Text,
    /// The computer's product family (`product_family`), as the stick
    /// shell read it.
    pub maker_family: faraday_text::Text,
    /// The computer's model (`product_name`), as the stick shell read it.
    pub maker_model: faraday_text::Text,
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

pub use create::create_steps;

/// What a press leaves for the tick after the frame that shows it (`docs/SPEED.md` §9).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Later {
    /// A press whose work is [`drawn_first`], or the backup's shares made
    /// again once a slider that changes them is let go.
    Press(Action),
    /// A key whose work is slow (`Faraday::key_slow`, in `lightning`).
    Key(KeyIn),
    Vault(vault::Work, bool),
}

use slow::{drawn_first, working_text};

pub use inbox::file_stem;
pub(crate) use inbox::{result_stem, stem};

impl App for Faraday {
    fn event(&mut self, event: Event) {
        self.drain_jobs();
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
        // A scroll or a wheel moves the region's offset and nothing else
        // (`docs/FRAMES.md` §3.4).
        let mut only_moved = false;
        match event {
            Event::Display(d) => self.display(d),
            Event::Touch { x, y, phase } => {
                self.input_now();
                self.typing.blink_from(self.now_ms);
                // Any input puts the idle lock off; the press that does
                // it does nothing else.
                if self.sheet == Some(Sheet::IdleWarn) {
                    if phase == TouchPhase::Down {
                        self.put_away_idle_warning();
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
                    self.put_away_idle_warning();
                } else if self.later_then_slow(k) {
                    // Drawn first, done on the tick after (`docs/SPEED.md`
                    // §9).
                    self.later_now();
                    self.later = Some(Later::Key(k));
                    self.worker.set_drawn(false);
                    let line = if crate::seed_check::here(self) {
                        working_text(Action::SeedCheck(seed_check::SeedAction::UsePass))
                            .unwrap_or(EN.working_node_key)
                    } else {
                        self.lightning_working()
                    };
                    self.working(line);
                    self.out.redraw();
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
                    self.put_away_idle_warning();
                } else {
                    self.mouse_back();
                }
            }
            Event::Forward => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.put_away_idle_warning();
                } else {
                    self.mouse_forward();
                }
            }
            Event::Scroll { x, y, dy } => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.put_away_idle_warning();
                }
                // A gesture goes on moving the region it started over.
                if self.pointer.scroll_at().is_none() {
                    self.point_scroll(i32::from(x), i32::from(y), f32::from(dy));
                }
                // Pixels the content moves up, as the shell API states
                // them, in this layout's units.
                self.pan(f32::from(dy) / self.f.max(0.1));
                self.pointer.set_scroll_at(Some(self.now_ms));
                only_moved = true;
            }
            Event::Wheel { x, y, dy } => {
                self.input_now();
                if self.sheet == Some(Sheet::IdleWarn) {
                    self.put_away_idle_warning();
                }
                self.point_region(i32::from(x), i32::from(y));
                // The shells send 48 pixels a notch; the notch's distance
                // is this layout's, in units, so it covers the same amount
                // of page on every screen.
                self.wheel(f32::from(dy) / motion::SHELL_NOTCH_PX * motion::WHEEL_NOTCH);
                only_moved = true;
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
                if self.out.toast.is_some() && self.out.toast_at.is_none() {
                    self.out.toast_at = Some(now_ms);
                }
                let fading = fading
                    || self.typing.caret_changed(self.caret_on())
                    || self.disclosed.is_some_and(|d| d.moving)
                    || self.busy().is_some()
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
                let tip = self.pointer.tip_tick(now_ms)
                    | self.keygen.dice_tick(now_ms)
                    | kef_tool::tick(self);
                let (own, motion) = (self.own_tick(now_ms), self.motion_tick(now_ms));
                if own || fading || tip {
                    self.out.redraw();
                } else if motion {
                    self.out.moved();
                }
                if own || motion || fading || tip {
                    self.out.commands.push_back(Command::Draw);
                }
                // A fade under way asks for its next step; unless
                // something else asks for a frame too, the step blends
                // the two kept frames (`Faraday::fade_step`).
                if self.fade.tick_due() {
                    self.out.commands.push_back(Command::Draw);
                }
                if self.idle_tick(now_ms) {
                    self.out.redraw();
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
                    | address_check::tick(self, now_ms)
                    | self.chart_side_tick(now_ms)
                    | self.wallet_sheet_open()
                    | self.published_sheet_open()
                    | self.boot_notice_tick(now_ms)
                {
                    self.out.redraw();
                    self.out.commands.push_back(Command::Draw);
                }
                // A live clock or a TOTP code on screen moves on by the
                // second, with nothing else moving.
                if self.shows_clock() && self.clock_now() != self.clock_drawn {
                    self.clock_drawn = self.clock_now();
                    self.out.redraw();
                    self.out.commands.push_back(Command::Draw);
                }
                self.vanity_tick(now_ms);
                self.zero_tick();
                if self.scan_tick(now_ms) | self.qr.tick(now_ms, self.settings.qr_frame_ms()) {
                    self.out.redraw();
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
            } => {
                let luma = zeroize::Zeroizing::new(luma);
                let chroma = chroma.map(zeroize::Zeroizing::new);
                if !self.camera.frame_in(width, height, luma, chroma) {
                    return;
                }
            }
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
            Event::Clipboard { text, .. } => self.pasted(text),
            Event::ClipboardUnavailable { .. } => self.pasted(faraday_text::Text::new()),
            Event::RotationOffered { native } => self.rotation_offered_by(native),
            Event::ExternalScreen => self.external_screen(),
            _ => return,
        }
        if settle {
            self.event_settled();
            self.after_input();
        }
        if only_moved {
            self.out.moved();
        } else {
            self.out.redraw();
        }
        self.out.commands.push_back(Command::Draw);
    }

    fn parts(&mut self) -> faraday_shell_api::Parts<'_> {
        self.parts_moved()
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.note_secrets();
        self.out.commands.pop_front()
    }

    fn frame(&mut self) -> Frame<'_> {
        if let Some(only_moved) = self.out.take_wanted() {
            self.before_frame();
            self.render(only_moved);
        } else if self.fade.is_due() {
            self.fade_step();
            self.out.kept.whole_frame();
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

/// Whether a pass over this transaction may open a MuSig2 round, which
/// draws secret nonces: any such input is one, since what the pass does
/// with it is decided only as it signs.
pub(crate) fn spend_opens_round(inspection: &faraday_psbt::Inspection) -> bool {
    inspection.inputs.iter().any(|i| i.musig.is_some())
}
