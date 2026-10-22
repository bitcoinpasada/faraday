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
//!   storage events), `version`, `action` (the `Action` enum), `qr_view`
//!   (the QR view and the camera's scan state), `receipt` and `state` (the
//!   state of Entry, Message and the stick visit). Each other area's state
//!   is in its own `mod.rs` or file: `DecodeState` in `decode`,
//!   `SpendState` in `wallet`, `BackupState` in `wallet_backup`,
//!   `CreateState` in `create`, `RestoreState` in `restore`. `lib.rs`
//!   re-exports them all, so a caller names `faraday_core::Action`.
//! - State and actions: `work` (costly work counted), `compute` (where
//!   event code gets its token), `flow`, `inputs`, `edit` (the caret and
//!   the selection), `motion` (scrolling), `fade` (a change fading
//!   in), `menu` (the ⋮ menus),
//!   `file_menu`, `in_progress`, `compact` (the small panel), `medium`
//!   (what the removable medium is called), `memory` (what is kept across
//!   a lock), `copy_check` (a copy by hand checked), `set_time`, `stick`
//!   (`pills.rs`, `settings.rs`, `wipe.rs`), `visit_taken`, `trail` (where
//!   Back lands: the screens left, as they were seen), and `ui` (drawing
//!   in design units).
//! - Screens and drawing: `screens` (the frame, the sidebar, the sheets and
//!   the screens with no area folder), `drawing`, each area's `screen.rs`
//!   (`vault/screen.rs`, `keygen/screen.rs`), `glance` (a wallet at a
//!   glance), `guide` (Guided mode's walk-through), `scan_key`, `pdf` (the
//!   printed sheets), `picture` (a QR code to keep), `demo` (what Demo
//!   refuses), `devices` (what else captures the screen or the keys) and
//!   `edid` (what a screen says it is).
//! - Flows around a key, seed, words or passphrase: `keygen`, `seeds`
//!   (with `seeds/read.rs`), `wallet`, `inbox`, `create`, `create_from`,
//!   `bip85`, `forms`, `fresh`, `gpg`, `apk`, `secure_boot`, `lightning`,
//!   `silent`, `vanity`, `grind`, `paper`, `quiz`, `secrets`,
//!   `word_entry`, `vault` (with `vault/load.rs` and `vault/out.rs`),
//!   `chart_edit`, `boot_import` and `backup_flow/wallet.rs`. They hold
//!   handles and public results; key use is `faraday-keys` (F70,
//!   `docs/KEYS.md`), and `tools/lint-secrets.sh` rule 4 holds it.
//! - Wallets and PSBTs: `plan`, `decode`, `family`, `explore`, `catalog`.
//! - Backups and the backup flow: `backup_flow`, `backups`, `board`,
//!   `held` (the backup before spending), `restore`, `wallet_backup`.
//! - Vaults: `codes` (TOTP codes); `vault` is listed with the flows above.
//! - Sticks, boot and files: `upgrade`, `transfer` (the desktop as the
//!   device's QR link), `names` (the Starter's install names),
//!   `secure_boot/state.rs`.
//! - Support: `wordlist`, `learn`, `modes` (F81, with `modes/screen.rs`), `samples` (with `samples/screen.rs`), `tools`, `testkit`.
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
//! `mod.rs` holds the frame and the dispatch, `sidebar.rs` the sidebar,
//! `sheets.rs` the sheets and `parts.rs` what two or more screens share.
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
//! | `Catalog` | `screens/catalog.rs` |
//! | `Transfer` | `transfer/screen.rs` |
//! | `Upgrade` | `upgrade/screen.rs` |
//! | `BackupFlow` | `backup_flow/screen.rs` |

// The shell's protocol and the app's types: storage, version, actions, QR view, receipt, state.
mod action;
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
pub mod in_progress;
pub mod inputs;
pub mod medium;
pub mod memory;
mod menu;
mod motion;
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
mod screens;

// Flows around a key, seed, words or passphrase; key use is faraday-keys (F70).
pub mod apk;
pub mod bip85;
pub mod boot_import;
pub mod chart_edit;
pub mod create;
mod create_from;
pub mod forms;
mod fresh;
pub mod gpg;
pub mod grind;
pub mod inbox;
pub mod keygen;
pub mod lightning;
pub mod paper;
pub mod quiz;
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
pub mod upgrade;

// Support: word lists, the Learn pages, the tools page, test data.
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
pub use medium::Medium;
pub use modes::{DESKTOP_MODES, Mode, STICK_MODES};
pub use qr_view::{
    Code, NoPicture, QR_PARTS, QR_SPEEDS, QrFormat, QrSource, QrView, SEEN_FRAMES, ScanPurpose,
    ScanState, Seen, TOO_FINE_TENTHS,
};
pub(crate) use qr_view::{TOO_FINE_PASSES, UrFed, send_error, ur_feed, ur_note};
pub(crate) use receipt::sha256_of;
pub use receipt::{Receipt, ReceiptFile};
pub use restore::RestoreState;
pub use screens::qr_code;
pub use secure_boot::state::SecureBootAtStart;
pub use state::{EntryState, MessageState, VisitState};
pub use stick::pills::{BOOT_FOLDER, FIRST_BACKUP, Pill, SticksWanted, backup_folder};
pub use storage_api::{BootPart, StickInfo, StorageCommand, StorageEvent};
pub use version::{
    BUILD, Program, VERSION, is_release, parse_version_output, version_label, version_line,
};
pub use wallet::SpendState;
pub use wallet_backup::BackupState;

use crate::strings::EN;
use crate::strings::fill;
use crate::wallet::KeyTexts;
use faraday_intake::{arrival, seed_in_code, seedqr_words};
use std::collections::{BTreeSet, VecDeque};

use faraday_crypto::work::Compute;
use faraday_draw::{Canvas, Rect};
use faraday_shell_api::{App, Command, DisplayInfo, Event, Frame, Key as KeyIn, TouchPhase};

use wallet::{FileKind, Session, Spend, fp_text};

/// Minutes without input before the session locks, at first (F58).
pub const IDLE_LOCK_MIN: u16 = 30;
/// Minutes without input before the machine powers off, at first,
/// counted from the last input across the lock (F58).
pub const IDLE_OFF_MIN: u16 = 60;
/// How long before an idle lock its warning comes up, at most: half the
/// lock time when that is shorter.
pub const IDLE_WARN_MS: u64 = 5 * 60_000;

/// The exit code that asks the app loop for a fresh process
/// (`docs/history/PLAN.md` §5.3).
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
    /// Setting the time by hand (`docs/TOTP.md` §3).
    SetTime,
    /// The vaults the Inbox and Outbox hold.
    Vaults,
    /// Every wallet known and where its backup is (`docs/history/SIMPLIFY.md`
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
    /// The calculators.
    Tools,
    /// Unlocking one slot of a vault.
    Unlock,
    /// An open vault's contents.
    VaultContents,
    /// TOTP codes for the entries in open vaults (`docs/TOTP.md` §5).
    Codes,
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
    if stick::settings::is_file(name) {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "psbt" => Some(EN.stick_kind_psbt),
        "txt" | "json" | "descriptor" => Some(EN.stick_kind_text),
        "txn" => Some(EN.stick_kind_transaction),
        "faraday-sheet" => Some(EN.stick_kind_sheet_for),
        "ofv" => Some(EN.stick_kind_vault),
        "efi" => Some(EN.stick_kind_efi_image),
        "osk" => Some(EN.stick_kind_threshold),
        "oskb" => Some(EN.stick_kind_oskb),
        "png" => Some(EN.stick_kind_qr_codes),
        _ => Some(EN.stick_kind_file),
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
    let m = faraday_keys::words::from_compact_seedqr(bytes, faraday_bip::bip39::Language::English)
        .ok()?;
    Some(zeroize::Zeroizing::new(m.indices().to_vec()))
}

/// Said on the camera when what it read would load a key with a stick
/// attached.
fn stick_keys(m: Medium) -> String {
    fill(EN.stick_keys_keys_load, &[m.noun()])
}

/// The most QR codes of public text kept ([`Faraday::public_qr`]); the
/// next one empties the list.
pub(crate) const PUBLIC_QRS_KEPT: usize = 8;
/// The most wallet descriptions kept read by their text
/// ([`Faraday::read_kept`]); the next one drops the oldest.
pub(crate) const TEXTS_READ_KEPT: usize = 32;
/// The most multisig configs' names kept ([`Faraday::config_name_kept`]);
/// the next one drops the oldest.
pub(crate) const CONFIG_NAMES_KEPT: usize = 16;
/// The most descriptions kept parsed as Create builds them
/// ([`Faraday::parsed_kept`]); the next one drops the oldest.
pub(crate) const TEXTS_PARSED_KEPT: usize = 32;
/// The most plans kept mapped ([`Faraday::plan_mapped`]): the backup's,
/// a flow's, a chart's; the next one drops the oldest.
pub(crate) const PLANS_MAPPED_KEPT: usize = 8;
/// The most seeds' fingerprints kept by a hash of the secret
/// ([`Faraday::fp_memo`]); the next one drops the oldest.
pub(crate) const FP_MEMO_KEPT: usize = 64;
/// The most first addresses of the Seeds tool's descriptions kept
/// ([`Faraday::seeds_address`]); the next one empties the list.
pub(crate) const SEEDS_ADDRESSES_KEPT: usize = 16;

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
    /// SHA-256 of an APK's, a vault's or an EFI image's bytes, taken once
    /// when the file came in (each is public: a vault's file is sealed);
    /// `None` for any other file. An Item's bytes never change after it
    /// is made.
    pub sha: Option<[u8; 32]>,
    /// The wallet a wallet file describes, read once when the file came
    /// in, as [`Item::sha`] is taken (`docs/SPEED.md` §3.3): `None` for
    /// any other file. An Item's bytes are never changed after it is
    /// made, so this stays what they say; nothing reads the bytes again
    /// for it.
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
    /// hash of an APK, a vault or an EFI image. The file as it came in,
    /// classified, with what it describes read once.
    fn new(c: &Compute, name: &str, bytes: Vec<u8>) -> Item {
        let wallet::Classified { kind, psbt, wallet } = wallet::classify_read(c, name, &bytes);
        let sha = matches!(kind, FileKind::Apk | FileKind::Vault | FileKind::EfiImage)
            .then(|| sha256_of(c, &bytes));
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
    /// bytes: the one taken when it came in ([`Item::sha`], an APK's, a
    /// vault's or an EFI image's), a lookup. Any other file is hashed now.
    pub fn hash(&self, c: &Compute) -> [u8; 32] {
        self.sha.unwrap_or_else(|| sha256_of(c, &self.bytes))
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
            FileKind::Carry | FileKind::Entries | FileKind::SecretWallet => Some(secrets::Ack::Any),
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
    /// A seed the camera read, asking how it loads: with no passphrase
    /// or with one (`seed_read`).
    SeedRead,
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
    /// A vault record's Save to a stick: its files, to tick, and Save
    /// (`docs/BACKUP-FLOW.md` §17.7).
    VaultSave,
    /// Cancel pressed in a flow whose ending loses something: what is
    /// lost, **Cancel the flow** and **Keep going** (F51).
    CancelFlow(in_progress::Going),
    /// A file row's ⋮ menu (`docs/FLOWS.md` decision 61): a popup under
    /// the ⋮ on `wide`, a sheet on the small panel.
    FileMenu,
    /// **Rename {name}**: a file For the stick, or one on the stick
    /// visited.
    Rename,
    /// A file's Details: name, kind, size and SHA-256.
    Details,
    /// **Load from {vault}**, after an unlock from the boot sheet or the
    /// Vaults tab: what to load, ticked (`vault_load`, F62).
    UnlockLoad,
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
    /// **Load samples**, from the Demo pill's neighbour (`samples/screen.rs`).
    Samples,
}

/// A description's first receive address on a network, or why there is
/// none ([`Faraday::seeds_address`]).
type KeptAddress = (String, faraday_bip::keys::Network, Result<String, String>);

impl Faraday {
    /// Costs a QR encoding the first time. The QR code of `text`, public
    /// text a screen shows (a guide's address): made the first time it is
    /// asked, by whoever asks, a frame included, and kept on the state by
    /// its text, a lookup after (`docs/SPEED.md` §2 rule 1: what is kept is
    /// public).
    pub(crate) fn public_qr(
        &self,
        c: &Compute,
        text: &str,
    ) -> Option<std::rc::Rc<faraday_codec::qr::QrMatrix>> {
        if let Some((_, m)) = self.public_qrs.borrow().iter().find(|(t, _)| t == text) {
            return m.clone();
        }
        let m = {
            faraday_codec::qr::encode(
                c,
                faraday_codec::qr::Payload::Bytes(text.as_bytes()),
                faraday_codec::qr::Ecc::Medium,
            )
            .ok()
            .map(std::rc::Rc::new)
        };
        let mut kept = self.public_qrs.borrow_mut();
        if kept.len() >= PUBLIC_QRS_KEPT {
            kept.clear();
        }
        kept.push((text.to_string(), m.clone()));
        m
    }
}

/// Costs a QR encoding (through `encode_bounded` and `bbqr_frames`).
/// Text as pictures keep it: one static code while it fits version 25
/// at medium error correction (the bound `docs/WALLETS.md` §5 sets for
/// a descriptor on screen), else the BBQr parts the QR view makes, one
/// code each, which a scanner that assembles parts takes one after the
/// other.
pub(crate) fn picture_codes(
    c: &Compute,
    text: &str,
) -> Result<Vec<faraday_codec::qr::QrMatrix>, String> {
    use faraday_codec::qr::{Ecc, Payload, encode_bounded};
    if let Ok(m) = encode_bounded(c, Payload::Bytes(text.as_bytes()), Ecc::Medium, 25) {
        return Ok(vec![m]);
    }
    faraday_qr::send::bbqr_frames(c, 'U', text.as_bytes(), QR_PARTS[1])
        .map(|(_, frames)| frames)
        .map_err(send_error)
}

/// The file names of the pictures [`picture_codes`] makes of `text`,
/// without making them: `{name}.png` for one code, else
/// `{name}{part}{j}-of-{k}.png` for each of the `k` parts, and none when
/// the text cannot be sent as parts. Costs the splitting of `text` into
/// BBQr parts when it needs more than one code, and no code is drawn.
pub(crate) fn picture_names(name: &str, part: &str, text: &str) -> Vec<String> {
    use faraday_codec::qr::{Ecc, byte_capacity};
    if text.len() <= byte_capacity(25, Ecc::Medium) {
        return vec![format!("{name}.png")];
    }
    let Some(parts) = faraday_qr::bbqr::encode('U', text.as_bytes(), QR_PARTS[1]) else {
        return Vec::new();
    };
    let k = parts.len();
    if k == 1 {
        return vec![format!("{name}.png")];
    }
    (1..=k)
        .map(|j| format!("{name}{part}{j}-of-{k}.png"))
        .collect()
}

/// Splits `text` into BBQr parts when it needs more than one code: what
/// each picture [`picture_codes`] makes of `text` holds, in the order
/// [`picture_names`] names them: the text itself for one code, else each
/// BBQr part. No code is drawn.
pub(crate) fn picture_payloads(text: &str) -> Vec<Vec<u8>> {
    use faraday_codec::qr::{Ecc, byte_capacity};
    if text.len() <= byte_capacity(25, Ecc::Medium) {
        return vec![text.as_bytes().to_vec()];
    }
    faraday_qr::bbqr::encode('U', text.as_bytes(), QR_PARTS[1])
        .map(|parts| parts.into_iter().map(String::into_bytes).collect())
        .unwrap_or_default()
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
        Some(p) => fill(EN.key_label_key_2, &[fp, kind, &p.to_string()]),
        None => fill(EN.key_label_key, &[fp, kind]),
    }
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

/// The progress of a multi-part code: parts read, and which are missing.
fn part_note(what: &str, have: usize, total: usize, missing: &[usize]) -> String {
    let list: Vec<String> = missing.iter().take(8).map(usize::to_string).collect();
    let more = if missing.len() > 8 { ", …" } else { "" };
    fill(
        EN.part_note_of_missing,
        &[
            what,
            &have.to_string(),
            &total.to_string(),
            &list.join(", ").to_string(),
            more,
        ],
    )
}

/// Costs what `secret_text` costs. Why scanned text is refused for
/// holding a secret: a seed's words or a private key.
fn secret_text(c: &Compute, text: &str) -> Option<&'static str> {
    faraday_intake::secret_text(c, text).map(|s| match s {
        faraday_intake::TextSecret::Words => EN.secret_text_those_are,
        faraday_intake::TextSecret::ExtendedPrivateKey => EN.secret_text_that_is,
    })
}

/// Costs what `read_psbt` and `read_wallet` cost. What a code is when a
/// SeedQR was wanted and something else came.
fn what_instead_of_a_seed(c: &Compute, text: &str) -> Option<&'static str> {
    let t = text.trim();
    if wallet::read_psbt(c, t.as_bytes()).is_some()
        || t.to_ascii_lowercase().starts_with("ur:crypto-psbt")
    {
        return Some(EN.what_instead_of_a_seed_that_is_a);
    }
    if faraday_qr::bbqr::is_part(t) || faraday_codec::ur::is_ur(t) {
        return Some(EN.what_instead_of_a_seed_that_is);
    }
    if wallet::read_wallet(c, t).is_ok() || crate::create::read_key(t).is_some() {
        return Some(EN.what_instead_of_a_seed_that);
    }
    None
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

/// A signed message checked, by what its check is worked out from
/// ([`Faraday::message_check`]).
struct KeptMessage {
    text: String,
    network: faraday_bip::keys::Network,
    wallets: Vec<String>,
    check: Option<std::rc::Rc<wallet::MessageCheck>>,
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
    /// Back up this wallet, opening Back up on the wallet just made.
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
pub const NOT_A_KEY: &str = EN.app_not_a_key_of_this;

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
    /// Write to sticks: one lock, then each stick by place.
    pub const STICKS: u8 = 10;
    /// Copy seed i by hand and check it: `COPY + i`, the seed by its
    /// place among the wallet's, below [`PASSPHRASE`].
    pub const COPY: u8 = 16;
    /// Seed i's passphrase on paper, checked: `PASSPHRASE + i`, below
    /// [`VAULT`].
    pub const PASSPHRASE: u8 = 72;
    /// Vault v made and filled: `VAULT + v`, the first vault `VAULT`.
    pub const VAULT: u8 = 128;

    /// Whether `n` is a seed's copy by hand.
    pub fn is_copy(n: u8) -> bool {
        (COPY..PASSPHRASE).contains(&n)
    }

    /// Whether `n` is a seed's passphrase on paper.
    pub fn is_passphrase(n: u8) -> bool {
        (PASSPHRASE..VAULT).contains(&n)
    }

    /// An item's number.
    pub fn of(item: Item) -> u8 {
        match item {
            Item::Templates => BLANK,
            Item::Copy(i) => COPY + i.min(usize::from(PASSPHRASE - COPY - 1)) as u8,
            Item::Passphrase(i) => PASSPHRASE + i.min(usize::from(VAULT - PASSPHRASE - 1)) as u8,
            Item::Vault(v) => VAULT.saturating_add(v.min(127) as u8),
            Item::SeedFiles => FILES,
            Item::Sheets => SHEETS,
            Item::PublicFiles => PUBLIC,
            Item::ShowDescriptor => SHOW,
            Item::Envelopes => ENVELOPE,
            Item::Sticks => STICKS,
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
            STICKS => Item::Sticks,
            n if n >= VAULT => Item::Vault(usize::from(n - VAULT)),
            n if n >= PASSPHRASE => Item::Passphrase(usize::from(n - PASSPHRASE)),
            n if n >= COPY => Item::Copy(usize::from(n - COPY)),
            _ => return None,
        })
    }
}

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
    hits: Vec<(Rect, Action)>,
    pressed: Option<(Action, Rect)>,
    commands: VecDeque<Command>,
    storage_out: VecDeque<StorageCommand>,
    dirty: bool,
    /// The second of the clock last drawn on a screen that shows it live
    /// (Now, a TOTP code and its seconds left), so a tick that moves it
    /// on draws again.
    clock_drawn: Option<u64>,
    restart: bool,
    /// The exit asked for a restart of the computer, not a power-off.
    restart_computer: bool,
    last_display: Option<DisplayInfo>,
    /// The person's display scale, in percent of the automatic one.
    pub scale_pct: u16,
    /// The palette the screens are drawn in.
    pub theme: ui::Theme,
    /// Reduce motion: nothing slides, glides, coasts or stretches.
    pub reduce_motion: bool,
    /// Minutes without input before the session locks; 0 for never
    /// (`docs/history/PLAN.md` §12.3).
    pub idle_lock_min: u16,
    /// How long each frame of an animated code stays up, in ms.
    pub qr_frame_ms: u64,
    /// Minutes without input before the machine powers off, when nothing
    /// is held and the Outbox is empty; 0 for never.
    pub idle_off_min: u16,
    /// The time zone, minutes east of UTC (`docs/TOTP.md` §2.1): −720 to
    /// +840, a multiple of 15.
    pub tz_min: i16,
    /// Setting the time by hand (`docs/TOTP.md` §3).
    pub set_time: set_time::SetTime,
    /// When the last input came, on this process's clock; set at the
    /// first tick.
    last_input: Option<u64>,
    /// Idle time the previous process had when it locked for idleness,
    /// counted toward power-off here.
    idle_carry: u64,
    /// This process locks for idleness, after this many ms without input.
    idle_locking: Option<u64>,
    /// The lock asked for last is a step of a backup flow (bringing in
    /// the setup's sticks, Lock and write), not the person's own Lock:
    /// it keeps Make a new wallet (`docs/history/NEW-WALLET.md` §18). Set by
    /// every way a lock is asked for.
    pub(crate) lock_for_flow: bool,
    /// The idle countdown's whole seconds last drawn, so a tick redraws
    /// only when the number on screen changes.
    idle_shown: Option<u64>,
    /// Whole minutes without input when the idle warning opened: what
    /// its text says, fixed while it is up.
    pub(crate) idle_warn_min: u64,
    /// A secret has been held or typed in this process. Only a fresh
    /// process is clean (`docs/history/PLAN.md` §5.1).
    tainted: bool,
    /// A stick arrived while the camera was on: held back, and taken as
    /// arriving once the camera is off.
    stick_held: bool,
    /// New input devices waiting to be believed (`docs/history/PLAN.md` §4.6).
    pub inputs: Vec<inputs::NewInput>,
    /// Devices kept out until unplugged, by id and name.
    pub ignored_inputs: Vec<(u32, String)>,
    /// Keyboards kept out this session for typing the wrong code.
    inputs_kept_out: usize,
    /// The boot import's sheet waits under the new-device sheet, and
    /// comes up once every waiting device is decided.
    import_under_input: bool,
    /// Files the boot import brought in with the stick still in, by name:
    /// the seeds they hold load once it is pulled (F60).
    pub(crate) import_load_after: BTreeSet<String>,
    /// Decisions the shell has not taken yet.
    input_decisions: VecDeque<(u32, bool)>,
    /// New keys made this session, which number their labels.
    new_keys: u32,
    /// The keys made here whose backup is not yet done, by fingerprint
    /// (`docs/history/NEW-WALLET.md` §14.3): they sign nothing until it is. Never
    /// a secret; kept in a vault beside the seed and in the plan record.
    made_here: Vec<faraday_bip::keys::Fingerprint>,
    /// What a frame works out once (`docs/SPEED.md` §3.3): on only while
    /// `render` runs.
    drawing: drawing::Drawing,
    /// QR codes of public text a screen shows (a guide's address), each
    /// made once, by its text ([`Faraday::public_qr`]).
    public_qrs: std::cell::RefCell<Vec<(String, Option<std::rc::Rc<faraday_codec::qr::QrMatrix>>)>>,
    /// Wallet descriptions read that no file or record keeps read, each
    /// by its text ([`Faraday::read_kept`]): public, and never stale.
    texts_read: std::cell::RefCell<Vec<(String, Option<faraday_bip::policy::WalletPolicy>)>>,
    /// The `Name:` line of each multisig config put together from split
    /// sheets ([`Faraday::config_name_kept`]), by its text: public, and
    /// never stale.
    config_names: std::cell::RefCell<Vec<(String, Option<String>)>>,
    /// Descriptions the Seeds tool puts together, each parsed as Create
    /// builds it ([`Faraday::parsed_kept`]), by its text: public, and
    /// never stale.
    texts_parsed:
        std::cell::RefCell<Vec<(String, Result<faraday_bip::policy::WalletPolicy, String>)>>,
    /// The signed message last checked ([`Faraday::message_check`]), by
    /// its text, the network and the loaded wallets' checksums: public,
    /// and never stale, since a change to any of them is another key.
    message_checked: std::cell::RefCell<Option<KeptMessage>>,
    /// Plans mapped, each by the shape and answers it is the map of
    /// ([`Faraday::plan_mapped`]): public, and never stale, since a
    /// change to either is another key.
    plans_mapped: std::cell::RefCell<Vec<(plan::Shape, plan::Answers, std::rc::Rc<plan::Mapped>)>>,
    /// First receive addresses of descriptions Seeds puts together, by the
    /// description and network ([`Faraday::seeds_address`]): public.
    seeds_addresses: std::cell::RefCell<Vec<KeptAddress>>,
    /// The last number given to a Make a new wallet or a Restore, which a
    /// wallet's backup flow is tied to while the wallet is being made.
    owner_seq: u64,
    /// The vaults: the open slots and the vault screens' state.
    pub vaults: vault::Vaults,
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
    /// checksum (`docs/history/SIMPLIFY.md` §5): kept across a lock, gone at
    /// power-off.
    pub backup_records: Vec<backups::Record>,
    /// The wallet the Backups screen is on, by its place in
    /// [`Faraday::backups_now`].
    pub backups_at: usize,
    /// What the last stick visit wrote, this power-on (§4.3).
    pub receipt: Option<Receipt>,
    /// Each vault file read from a stick this power-on: its name, the
    /// hash of its bytes as read, and the stick's label (§3.5).
    pub vault_from: Vec<(String, [u8; 32], String)>,
    /// The session's seed, once the shell's first answer arrived
    /// (`fresh.rs`).
    seed: Option<fresh::Seed>,
    /// When the wait for the seed started: the first tick after the
    /// display.
    seed_wait_from: Option<u64>,
    /// Whether the last frame said the seed is missing.
    seed_missing_drawn: bool,
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
    /// The open sheet's offset as the frame draws it.
    sheet_offset: f32,
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
    /// APK signing is in the app: off by default (F59).
    pub apk_signing: bool,
    /// Files read from sticks.
    pub inbox: inbox::InboxFiles,
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
    /// How far Files' For the stick list is scrolled on a desktop, apart
    /// from From the stick's (`list_offset`).
    pub files_out_offset: f32,
    /// How far the backup board's list column is scrolled, in design
    /// units (`docs/BACKUP-FLOW.md` §17.14).
    pub board_list_offset: f32,
    /// Design units scrolled past in Vault contents' item, on a desktop.
    pub vault_item_offset: f32,
    /// The wallet's chart is open as its own page, on a small panel.
    pub glance: bool,
    /// The chart and the thing on it whose sheet is open.
    pub chart: Option<(glance::Press, glance::sheet::Target)>,
    /// A chart's key whose lines are drawn bold, by place among its keys:
    /// **Where it is**.
    pub chart_focus: Option<(glance::Press, usize)>,
    /// What the chart's sheets hold between presses: a change asked for,
    /// a name or a passphrase typed (`docs/history/NEW-WALLET.md` §9.5–§9.7).
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
    /// The steps ticked on each guide page the Learn sheet has shown
    /// this session, by page; a page with none ticked is not here.
    /// Session state only, in memory, never written to a file.
    learn_ticks: Vec<(&'static faraday_learn::Page, Vec<u32>)>,
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
    scan_parts: Option<(faraday_codec::ur::Decoder, faraday_qr::Assembler)>,
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
    /// to (`docs/history/NEW-WALLET.md` §13.2).
    pub(crate) edit: ui::Edit,
    /// An edit inside a field is being typed through the field's own
    /// keys.
    editing: bool,
    /// Shift is held.
    shift: bool,
    /// Where the last frame drew each line of typing's characters.
    fields: Vec<ui::FieldAt>,
    /// The screen's back link, as the last frame drew it, with the
    /// screen it was drawn on: what [`Action::Back`] does first.
    back_link: Option<(Screen, Action)>,
    /// Where in `hits` the last frame drew the back link's press.
    back_hit: Option<usize>,
    /// The screens left for another, each as it was seen, the last left
    /// last: where Back lands ([`trail`], F77).
    pub(crate) trail: Vec<trail::Spot>,
    /// The screen as the input being handled found it, until the input
    /// is done ([`Faraday::trail_note`]).
    entered: Option<trail::Spot>,
    /// The input being handled went back: the screen it left is not
    /// put on the trail.
    backing: bool,
    /// Screens left by the mouse's back button, the last left last
    /// (`docs/history/NEW-WALLET.md` §13.1).
    forward: Vec<Screen>,
    /// The screen the last back or forward button landed on.
    landed: Option<Screen>,
    /// Seeds' fingerprints with no passphrase, by a hash of the secret:
    /// the Inbox's wallets and seeds are put together on every frame.
    pub(crate) fp_memo: std::cell::RefCell<Vec<([u8; 32], faraday_bip::keys::Fingerprint)>>,
    /// Wallets the Inbox describes left out of its next load, by
    /// descriptor.
    pub inbox_skip: BTreeSet<String>,
    /// A seed from the Inbox being made a wallet, or a sealed backup
    /// being opened.
    pub potential: Option<inbox::Potential>,
    /// When a passphrase was last typed on a sheet that shows the key it
    /// makes (the potential wallet's, a seed read's), until the key is
    /// worked out once typing has paused ([`motion::TYPED_PAUSE_MS`]).
    typed_at: Option<u64>,
    /// A seed the camera read, waiting on [`Sheet::SeedRead`].
    pub seed_read: Option<seeds::read::SeedRead>,
    /// What [`Sheet::Delete`] is choosing.
    pub delete_ask: Option<stick::wipe::DeleteAsk>,
    /// A file row's ⋮ menu, Rename and Details (`docs/FLOWS.md`
    /// decision 61).
    pub files_menu: file_menu::FileMenus,
    /// Zeroing a stick's free space or whole partition, under way.
    pub zeroing: Option<stick::wipe::Zeroing>,
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
    /// A press whose work waits for a frame (`docs/SPEED.md` §9), and
    /// whether a frame has been drawn since it was pressed.
    later: Option<Later>,
    later_drawn: bool,
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
    /// A change of screen or sheet fading in, and how fast this
    /// machine has lately shown a fade's steps (`fade.rs`).
    fade: fade::Fade,
    /// The screen and sheet the last frame drew, and the Learn sheet's
    /// page, so that a change is known at the next.
    last_drawn: Option<(ScreenKey, usize)>,
    /// The Guided switch's pill sliding: where it started, 0 on Steps
    /// only and 1 on Guided, and the tick it started at.
    guided_moving: Option<(f32, Option<u64>)>,
    /// What the start-up self-test found (`faraday_selftest`): run when the
    /// display first arrives, before any input. A failure blocks the app
    /// on a screen naming the check, with Exit its only control.
    selftest: Option<faraday_selftest::Outcome>,
    /// What can be pressed under it.
    hovered: Option<Action>,
    scanned: u32,
    /// Running on an online machine (the desktop app), where sheets are
    /// turned into PDFs. Never set on the device.
    pub online: bool,
    /// Held by another app that shows only Transfer and the Learn sheet
    /// (the Starter, `docs/STARTER.md` §2), set with [`Faraday::online`]:
    /// no sidebar and no Home of its own. Its way back goes to this
    /// app's Home, which draws nothing; the holder takes leaving
    /// Transfer and the sheets as going back to its own Home.
    pub held: bool,
    /// The shell lets a person insert and pull sticks by hand, and the
    /// app draws a pill for each across the top of the window
    /// ([`Faraday::stick_pills`]): the desktop app and the website demo.
    /// Never set on the device.
    pub shell_sticks: bool,
    /// The shell answers `RequestClipboard` with the clipboard's text: the
    /// desktop app. The scanner's Paste is dimmed where it is not set,
    /// which is never on the device (`docs/QR.md` §8 item 5).
    pub clipboard: bool,
    /// Pills pressed since the shell last asked: the folder names to
    /// insert or pull ([`Faraday::take_stick_toggles`]).
    pub(crate) stick_toggles: Vec<String>,
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
    next_mode: Option<Mode>,
    /// The samples Demo offers, set by the desktop; none elsewhere.
    pub samples: Option<&'static samples::Samples>,
    /// The Load samples sheet's rows (`samples::SheetState`).
    sample_sheet: samples::SheetState,
    /// The backup flows, at most one on screen (`docs/BACKUP-FLOW.md` §7).
    pub backup_flows: Vec<backup_flow::BackupFlow>,
    /// The flow on screen.
    pub(crate) flow_on: Option<usize>,
    /// The sticks the shell found unusable (not FAT32, read-only), by id.
    pub(crate) stick_problems: Vec<(String, bool)>,
    /// The flows begun, in the order they were begun, each with the screen
    /// of its own it was last on (`docs/FLOWS.md`, Flows in progress):
    /// only the order and the way back, worked out after each press.
    pub(crate) flows_begun: Vec<(in_progress::Going, Screen)>,
}

impl Default for Faraday {
    fn default() -> Self {
        Self::new()
    }
}

impl Faraday {
    /// A fresh app: no keys, empty boxes, Home.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
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
            clock_drawn: None,
            restart: false,
            restart_computer: false,
            last_display: None,
            scale_pct: 100,
            theme: ui::Theme::default(),
            reduce_motion: false,
            idle_lock_min: IDLE_LOCK_MIN,
            qr_frame_ms: QR_SPEEDS[0],
            idle_off_min: IDLE_OFF_MIN,
            tz_min: 0,
            set_time: set_time::SetTime::default(),
            last_input: None,
            idle_carry: 0,
            idle_locking: None,
            lock_for_flow: false,
            idle_shown: None,
            idle_warn_min: 0,
            tainted: false,
            stick_held: false,
            inputs: Vec::new(),
            ignored_inputs: Vec::new(),
            inputs_kept_out: 0,
            import_under_input: false,
            import_load_after: BTreeSet::new(),
            input_decisions: VecDeque::new(),
            new_keys: 0,
            made_here: Vec::new(),
            drawing: drawing::Drawing::default(),
            public_qrs: Default::default(),
            seeds_addresses: Default::default(),
            plans_mapped: Default::default(),
            texts_read: Default::default(),
            message_checked: Default::default(),
            texts_parsed: Default::default(),
            config_names: Default::default(),
            owner_seq: 0,
            // A new GPG key's term starts at GnuPG's default, never at "never".
            vaults: {
                let mut v = vault::Vaults::default();
                v.gpg_years = gpg::DEFAULT_YEARS;
                v
            },
            signed_amounts: Vec::new(),
            seal_amounts: true,
            checked_wallets: Vec::new(),
            backup_records: Vec::new(),
            backups_at: 0,
            receipt: None,
            vault_from: Vec::new(),
            seed: None,
            seed_wait_from: None,
            seed_missing_drawn: false,
            sign_draws: 0,
            pool: Vec::new(),
            now_ms: 0,
            toast: None,
            screen: Screen::Home,
            sheet: None,
            sheet_scroll: (0.0, None),
            sheet_offset: 0.0,
            osk: compact::Osk::default(),
            session: Session::default(),
            spend: None,
            decode: None,
            catalog_find: String::new(),
            apk_signing: false,
            inbox: inbox::InboxFiles::default(),
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
            // Steps only is a choice (`docs/history/SIMPLIFY.md` §6.1). A saved
            // value, from the kept state or the boot stick's settings
            // file, overwrites this before the person sees a screen.
            guided: true,
            about_open: None,
            pin_h: std::cell::Cell::new(0.0),
            list_offset: 0.0,
            card_offset: 0.0,
            files_out_offset: 0.0,
            board_list_offset: 0.0,
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
            learn_ticks: Vec::new(),
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
            back_hit: None,
            trail: Vec::new(),
            entered: None,
            backing: false,
            forward: Vec::new(),
            landed: None,
            fp_memo: Default::default(),
            inbox_skip: BTreeSet::new(),
            potential: None,
            typed_at: None,
            seed_read: None,
            delete_ask: None,
            files_menu: Default::default(),
            zeroing: None,
            last_tap: None,
            down_at: (0, 0),
            drag: None,
            swallow: false,
            bar_held: None,
            caret_at: 0,
            caret_drawn: None,
            later: None,
            later_drawn: false,
            scroll_at: None,
            hover: None,
            toast_at: None,
            disclosed: None,
            frost: None,
            fade: fade::Fade::default(),
            last_drawn: None,
            guided_moving: None,
            selftest: None,
            hovered: None,
            online: false,
            held: false,
            shell_sticks: false,
            clipboard: false,
            stick_toggles: Vec::new(),
            medium: Medium::Stick,
            secure_boot: SecureBootAtStart::Unknown,
            maker: String::new(),
            maker_family: String::new(),
            maker_model: String::new(),
            devices: devices::DeviceCheck::default(),
            next_mode: None,
            samples: None,
            sample_sheet: samples::SheetState::default(),
            backup_flows: Vec::new(),
            flow_on: None,
            stick_problems: Vec::new(),
            flows_begun: Vec::new(),
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

    /// Whether the exit asks for the computer to restart rather than
    /// power off (`docs/FLOWS.md`, Restart): the shell ends the app with
    /// [`RESTART_COMPUTER_CODE`].
    pub fn computer_restart_requested(&self) -> bool {
        self.restart_computer
    }

    /// Whether the power sheet offers Restart: the PC stick and the Pi,
    /// not Faraday Desktop, an app on someone's own system.
    pub fn offers_restart(&self) -> bool {
        !self.shell_sticks
    }

    /// Whether a storage request is waiting for the shell (a lookup).
    pub fn storage_pending(&self) -> bool {
        !self.storage_out.is_empty()
    }

    /// The screen's name, and the sheet's when one is up, for a shell's
    /// slow-frame line: `Backup` or `Backup / Qr` (a small string).
    pub fn screen_name(&self) -> String {
        match self.sheet {
            Some(sheet) => format!("{:?} / {:?}", self.screen, sheet),
            None => format!("{:?}", self.screen),
        }
    }

    /// The next storage request, or `None`.
    pub fn poll_storage(&mut self) -> Option<StorageCommand> {
        self.storage_out.pop_front()
    }

    /// Delivers what the shell knows about storage.
    pub fn storage(&mut self, event: StorageEvent) {
        let screen = self.screen;
        self.note_here();
        self.storage_event(event);
        self.backup_settle();
        if self.screen != screen {
            // A stick that takes the person to another screen leaves
            // this one as a press would.
            self.after_input();
        } else {
            self.entered = None;
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
                let item = Item::new(&c, &name, bytes);
                if item.kind == FileKind::Vault
                    && let Some(s) = self.sticks.iter().find(|s| s.id == stick)
                {
                    let label = s.label.clone();
                    self.vault_read_from(&name, &item.bytes, &label);
                }
                // The visit's rows say what came in; only a seed waiting
                // for the stick's removal is worth a line.
                let line = (item.kind == FileKind::Words && self.visit.load_after.contains(&name))
                    .then(|| {
                        fill(
                            EN.storage_event_copied_a,
                            &[&name.to_string(), self.medium.noun()],
                        )
                    });
                self.inbox.retain(|i| i.name != name);
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
                self.files_menu.renamed.remove(&name);
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
                            self.inbox
                                .push(Item::new(&c, &item.name, item.bytes.clone()));
                        }
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
                self.receipt_add(&name, Some(reason.clone()));
                let stays = if self.visit.writing_inbox.remove(&name) {
                    EN.storage_event_it_stays.to_string()
                } else {
                    fill(EN.storage_event_it_still, &[self.medium.for_the()])
                };
                self.visit.log.push((
                    fill(
                        EN.storage_event_not_written,
                        &[&name.to_string(), &reason.to_string(), &stays.to_string()],
                    ),
                    false,
                ));
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
                let now_ms = self.now_ms;
                if let Some(s) = self.scan.as_mut() {
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
                                    s.note = Some(EN.scan_hold_it_square_on.to_string());
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
                self.inbox = inbox
                    .into_iter()
                    .map(|(n, b)| Item::new(&c, &n, b))
                    .collect();
                self.outbox = outbox
                    .into_iter()
                    .map(|(n, b)| Item::new(&c, &n, b))
                    .collect();
                self.restore_kept(&kept);
            }
        }
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    /// Whether anything secret is being typed: seed words, a BIP-39 or
    /// vault passphrase, a prompt's line. A secret sheet for an Inbox file
    /// on its way to another stick does not count: it holds a copy of
    /// what the Inbox already holds, and the stick it goes to must stay
    /// open (`docs/history/PLAN.md` §5.4).
    fn secret_typed(&self) -> bool {
        self.keygen.is_some()
            || self.secret_out.as_ref().is_some_and(|o| !o.to_visit)
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
        (self.idle_off_min > 0
            && !self.online
            && !self.lockable()
            && self.outbox.is_empty()
            && !self.flow_screen())
        .then(|| u64::from(self.idle_off_min) * 60_000)
    }

    /// The idle timers (`docs/history/PLAN.md` §12.3). Returns whether the screen
    /// changed.
    fn idle_tick(&mut self, now_ms: u64) -> bool {
        let c = self.compute();
        let since = *self.last_input.get_or_insert(now_ms);
        let idle = now_ms.saturating_sub(since) + self.idle_carry;
        let min = 60_000u64;
        // The warning, with its countdown, before the lock.
        // Make a new wallet before its keys are kept anywhere: no idle
        // lock at all (`docs/history/NEW-WALLET.md` §18).
        let held = self.lock_held(&c);
        if let Some((warn, lock)) = self.idle_lock_at()
            && idle >= warn
            && idle < lock
            && self.lockable()
            && self.idle_locking.is_none()
            && !held
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
            && !held
        {
            self.idle_locking = Some(idle);
            // No one is there to ask: the lock goes ahead, and an open
            // vault's changes with no seed to seal them are lost, as the
            // warning before it said.
            self.lock_now();
            return true;
        }
        if self.idle_off_min > 0
            && idle >= u64::from(self.idle_off_min) * min
            && !self.online
            && !self.lockable()
            && !self.flow_screen()
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
        let present: Vec<String> = self.sticks.iter().map(|s| s.id.clone()).collect();
        // What the shell found unusable comes with each listing: a stick
        // that no longer reports a problem has none.
        let _ = &present;
        self.stick_problems.clear();
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
        self.flow_sticks_changed();
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
                self.vaults.focus = Some(vault::Focus::Passphrase);
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
                    self.vault_act(vault::VaultAction::Open(i));
                }
            }
            if had {
                let loaded = self.load_after_pull() + self.import_load_after_pull();
                let said = if loaded == 0 {
                    fill(EN.sticks_changed_removed_2, &[self.medium.cap()])
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
                if let Some(a) = self.pull.take() {
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
            self.stick_held = true;
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
            self.vaults.focus = None;
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
            self.visit.out = self.visit_default_out(&c);
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
                && b.files.iter().any(|(n, _)| !stick::settings::is_file(n))
            {
                self.import_start(b);
            }
            // The boot stick's settings, once a session (`stick_settings`).
            if self.stick_settings.is_none()
                && let Some(boot) = self.sticks.iter().find(|s| s.boot)
            {
                match boot.files.iter().find(|(n, _)| stick::settings::is_file(n)) {
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
        let text = fill(EN.toast_out_waits, &[what, self.medium.for_the()]);
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
            Action::Scan
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
            Action::PotentialOpen(_) => EN.pull_what_load_the,
            Action::BackupOpen(_) => EN.pull_what_open_the,
            // Unlocking for another flow: the Pull sheet asks first, and
            // Unlock opens once the stick is out.
            Action::Vault(vault::VaultAction::OpenFrom(i, _))
                if self.vault_files().get(i).is_some_and(|f| f.open.is_none()) =>
            {
                EN.pull_what_unlock
            }
            Action::Vault(vault::VaultAction::ListFrom(_)) => EN.pull_what_unlock,
            Action::Flow(
                backup_flow::steps::FlowAction::UnlockVault(_)
                | backup_flow::steps::FlowAction::UnlockDone(_),
            ) => EN.pull_what_unlock,
            Action::Flow(
                backup_flow::steps::FlowAction::CreateVault(_)
                | backup_flow::steps::FlowAction::Make
                | backup_flow::steps::FlowAction::CopyKey,
            ) => EN.pull_what_make_a_key,
            Action::Catalog(i) => match catalog::TILES.get(usize::from(i))?.go {
                Go::VaultCategory(c)
                    if self.vaults.open.is_empty()
                        && !self.vault_files().is_empty()
                        && backup_flow::Kind::of_category(c).is_none() =>
                {
                    EN.pull_what_unlock
                }
                Go::NewKey | Go::NewShares => EN.pull_what_make_a_key,
                Go::AddKey(_) | Go::SeedQr => EN.pull_what_add_a_key,
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
        self.dirty = true;
    }

    /// The toast shown, if one is current.
    pub fn toast_text(&self) -> Option<&str> {
        self.toast.as_ref().map(|(t, _)| t.as_str())
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
        if !*started {
            *started = true;
            self.receipt = Some(Receipt {
                label: label.clone(),
                stick: stick.clone(),
                time: now,
                files: Vec::new(),
                by_flow: false,
            });
        }
        if sent.is_empty() {
            self.visit.writing = None;
        }
        let hash = self.hash_to_keep(&c, hash);
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

    /// The stick visit's chip (`docs/history/SIMPLIFY.md` §4.7): where pulling
    /// the stick goes. Back to the flow the stick came in during, Unlock
    /// when a vault was sealed for the visit, else Home.
    pub fn visit_then(&self) -> String {
        match self.after_visit {
            Some(s) => fill(EN.visit_then_then, &[self.back_name(s)]),
            None if self.visit_unlocks().is_some() => EN.visit_then_then_unlock.to_string(),
            None => EN.visit_then_then_home.to_string(),
        }
    }

    /// Remembers that vault file `name`, these bytes, was read from the
    /// stick labelled `label`.
    pub(crate) fn vault_read_from(&mut self, name: &str, bytes: &[u8], label: &str) {
        let c = self.compute();
        self.vault_from.retain(|(n, ..)| n != name);
        self.vault_from
            .push((name.to_string(), sha256_of(&c, bytes), label.to_string()));
    }

    /// What a visit ticks to come in (`docs/history/SIMPLIFY.md` §4.6): every file
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
                self.receipt
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

    /// Whether a lock would lose an open vault's changes: the session's
    /// seed, which every seal's nonce is drawn from, has not arrived.
    pub fn seal_waits(&self) -> bool {
        !self.seeded() && self.vaults.open.iter().any(|v| v.changes > 0)
    }

    /// Lock asked for: by the person (`for_flow` false), or by a step of
    /// a backup flow. Asks first when a vault is open with something
    /// not in it, a seal waits, or the lock would end Make a new wallet
    /// (`docs/history/NEW-WALLET.md` §18); else locks.
    pub(crate) fn lock_ask(&mut self, for_flow: bool) {
        let c = self.compute();
        self.lock_for_flow = for_flow;
        let (keys, wallets) = self.unsaved_now(&c);
        let sealed = self.vaults.open.iter().any(|v| v.changes > 0);
        if self.lock_ends_create(&c)
            || self.seal_waits()
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
        self.lock_held(c) && !self.lock_for_flow
    }

    /// Lock, unless it would lose an open vault's changes: then nothing
    /// is sealed or wiped, the vaults stay open with their changes, and
    /// the Lock sheet says what a lock without sealing loses.
    fn lock(&mut self) {
        if self.seal_waits() {
            self.commands.push_back(Command::RequestEntropy);
            self.sheet = Some(Sheet::LockAsk);
            return;
        }
        self.lock_now();
    }

    /// The state could not be kept. A lock that would lose the keys that
    /// wait for their backup does not go ahead.
    fn not_kept(&mut self) {
        if self.restart && self.backup_flows.iter().any(|f| f.waiting()) {
            self.restart = false;
            self.next_mode = None;
            self.toast(EN.toast_lock_not_kept);
        }
    }

    /// Lock: what can be sealed is, and the session goes.
    fn lock_now(&mut self) {
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
        self.import = None;
        self.import_load_after.clear();
        // Nothing secret copied in outlives the process that read it
        // (`docs/history/PLAN.md` §5.3): the next process is clean, and these would be
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
        let c = self.compute();
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        // A carry file is the PSBT and the secret nonce of the share that
        // signs next.
        let (psbt, carry) = if item.kind == FileKind::Carry {
            match faraday_keys::carry::parse(&item.bytes) {
                Ok((section, bytes)) => match faraday_psbt::Psbt::parse_bytes(&c, &bytes) {
                    Ok(p) => (p, Some(section)),
                    Err(_) => {
                        self.toast(EN.start_spend_the_carry);
                        return;
                    }
                },
                Err(e) => {
                    self.toast(e.reason());
                    return;
                }
            }
        } else {
            let Some(psbt) = wallet::read_psbt(&c, &item.bytes) else {
                self.toast(EN.start_spend_not_a_psbt);
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
        // Inspected once here; `refresh_spend` below finds it current.
        let inspection = self.session.inspect_with(&c, &spend.psbt, carry.as_ref());
        let inspected = Some((
            spend.psbt.clone(),
            self.session.inspect_inputs(),
            carry.is_some(),
        ));
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
            inspected,
            signers: Vec::new(),
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
        let c = self.compute();
        if let Some(s) = self.spend.as_mut() {
            // Inspected once each time the PSBT or the session changes,
            // not on every press that refreshes the spend.
            let inputs = self.session.inspect_inputs();
            if !s.inspected_for(&inputs) {
                s.inspection = self
                    .session
                    .inspect_with(&c, &s.spend.psbt, s.carry.as_ref());
                s.inspected = Some((s.spend.psbt.clone(), inputs, s.carry.is_some()));
            }
            s.signers = s
                .spend
                .signers_by(&c, &self.session, Some(&s.inspection))
                .iter()
                .map(|f| f.0)
                .collect();
            s.wallet = self.session.wallet_for(&c, &s.spend.psbt);
            let wallet = s.wallet.and_then(|w| self.session.wallets.get(w));
            let kind = wallet.map(|w| wallet::Kind::of(w.policy()));
            s.needed = wallet.map(wallet::needed).unwrap_or(1);
            // With no wallet loaded the signers come from the PSBT alone,
            // and a second device may still be needed.
            let kind = kind.unwrap_or(if s.inspection.participating_keys.len() > 1 {
                wallet::Kind::Multi(faraday_bip::policy::Wrapper::Wsh)
            } else {
                wallet::Kind::Single(faraday_bip::keys::ScriptType::NativeSegwit)
            });
            s.steps = kind.steps();
            s.complete = s.spend.finished.is_some() || s.spend.finish().is_ok();
            // Check is done already, this power-on, when this wallet was
            // checked before (`docs/WALLETS.md` §4): the flow opens on
            // the next card without a default instead.
            if let Some(wallet) = wallet {
                let checksum = wallet.checksum().to_string();
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

    /// Costs a hash of a whole file (through `vault_changed`). The session
    /// strip's stage, computed, never stored: where the person is in the
    /// lock cycle now. A stick attached while working still asks to bring
    /// it in, because the lock sheet is what asks to lock it; `Write out`
    /// is files For the stick, or a vault changed since it was written
    /// (§3.5), with no stick attached.
    pub fn session_stage(&self, c: &Compute) -> &'static str {
        if !self.sticks.is_empty() || self.import.is_some() {
            EN.session_stage_bring
        } else if !self.outbox.is_empty() || self.vault_changed(c).is_some() {
            EN.session_stage_write
        } else if self.holds_secret() || !self.session.wallets.is_empty() {
            EN.session_stage_work
        } else {
            EN.session_stage_open
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
        self.spend.as_ref().map(|s| s.needed).unwrap_or(1)
    }

    fn act(&mut self, action: Action) {
        // A choice in a file's ⋮ menu closes the menu first.
        self.file_menu_chosen(action);
        self.sample_sheet.pass_pressed = crate::demo::presses_pass_field(action);
        self.act_inner(action);
        // A wallet's flow follows the plan and the keys the press left.
        self.wallet_flow_sync();
        // A board left takes its empty sticks with it.
        self.board_tidy();
        // The order the flows were begun in, and where each was left.
        self.flows_note();
    }

    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles lock, power and idle-timer actions (`Lock*`, `NotNow`, `Power*`, `Restart`, `Idle*`).
    pub(crate) fn power_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::Lock => self.lock(),
            Action::LockUnsealed => self.lock_now(),
            Action::LockAsk => self.lock_ask(false),
            Action::NotNow => {
                self.next_mode = None;
                self.sheet = None;
                // Asked for by Upgrade, with no stick in: nothing waits.
                self.not_now = !std::mem::take(&mut self.upgrade_after_lock);
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
                    self.commands.push_back(Command::Exit);
                } else {
                    self.sheet = Some(Sheet::Power);
                }
            }
            Action::PowerOff => {
                self.sheet = None;
                self.commands.push_back(Command::Exit);
            }
            Action::Restart => {
                self.sheet = None;
                self.restart_computer = self.offers_restart();
                self.commands.push_back(Command::Exit);
            }
            Action::IdleLock(m) => {
                self.idle_lock_min = m;
                self.save_boxes();
            }
            Action::IdleOff(m) => {
                self.idle_off_min = m;
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
            Action::Cancel => {
                self.pull = None;
                self.vaults.stick_save = None;
                self.devices.after = None;
                self.devices.over_import = false;
                // A seed read and not loaded is wiped.
                self.seed_read = None;
                self.delete_ask = None;
                self.visit.taken.clear();
                self.next_mode = None;
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
                if let Some(scan) = self.scan.take() {
                    self.commands.push_back(Command::CameraOff);
                    // A copy's scan closed by the person: one scan without
                    // a match (a match closes the camera itself).
                    if let ScanPurpose::CheckCopy(fp) = scan.purpose {
                        self.copy_missed(fp.0);
                    }
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
                self.commands.push_back(Command::Exit);
            }
            return;
        }
        // In the demo an action that would make a secret does nothing:
        // no sheet, no lead, no toast (`docs/DEMO.md` §2, §7).
        if self.demo_refuses(action) {
            return;
        }
        // A stick's pill is the shell's, not the page's: it acts under
        // any sheet and waits for nothing.
        if let Action::StickPill(n) = action {
            self.stick_pill_pressed(n);
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
            self.chart = None;
        }
        // While something that can capture the screen or the keys is
        // attached and not let through, what shows a secret waits under
        // its sheet (`docs/STARTER.md` §3.1a).
        if self.device_holds(action) {
            // Reminders alone ride on the sheet that asks for the stick
            // to be pulled: one sheet, and pulling it is going on.
            let pull = self.selftest_passed()
                && !self.sticks.is_empty()
                && self.pull_what(action).is_some();
            if pull && self.devices.held.is_empty() {
                self.pull = Some(action);
                self.sheet = Some(Sheet::Pull);
                return;
            }
            self.device_hold(action);
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
            Action::Learn => self.learn_open(),
            Action::LearnShares => self.learn_shares(),
            Action::LearnExtraCopies => self.learn_on(faraday_learn::extra_copies_page()),
            Action::LearnSpend => {
                self.sheet = None;
                self.screen = Screen::Family;
            }
            Action::Osk(p) => self.osk_press(p),
            a @ (Action::ScanPaste
            | Action::ScanCamera(..)
            | Action::ScanPart
            | Action::ScanSeed
            | Action::ScanKey(..)
            | Action::Scan) => self.scan_act(a),
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
            | Action::SecretAck
            | Action::SecretUnprotected
            | Action::SecretForm(..)) => self.secret_act(a),
            Action::LearnPage(i) => self.learn_page(i),
            Action::LearnStep(n) => self.learn_tick_step(n),
            Action::WordList(a) => self.wordlist_act(a),
            Action::KeyGen(_)
            | Action::KStep(_)
            | Action::KStart(_)
            | Action::KOthers
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
            Action::Back => self.back(),
            Action::Resume(g) => self.flow_resume(g),
            Action::CancelFlow => self.flow_cancel(),
            Action::CancelFlowNow => self.flow_cancel_now(),
            a @ (Action::TransferSend(..)
            | Action::TransferReceive
            | Action::TransferOpenFolder) => self.transfer_act(a),
            Action::SeedRead(c) => self.seed_read_act(c),
            a @ (Action::DeleteFiles
            | Action::DeleteWhat(..)
            | Action::DeletePick(..)
            | Action::DeletePasses(..)
            | Action::DeleteGo
            | Action::ZeroStop) => self.wipe_act(a),
            Action::OpenInstitute => {
                if self.online {
                    self.storage_out.push_back(StorageCommand::OpenInstitute);
                }
            }
            a @ (Action::UpgradeOpen
            | Action::UpgradePick(..)
            | Action::UpgradeWrite
            | Action::UpgradeAgain
            | Action::UpgradeDone) => self.upgrade_act(a),
            a @ (Action::BackupsOf(..) | Action::BackupsPage(..)) => self.backups_act(a, &c),
            Action::Nav(s) => {
                self.chart_focus = None;
                if s == Screen::Backups {
                    self.backups_at = 0;
                }
                // Spend's way back is to where it was opened from.
                if s == Screen::Family && self.screen != Screen::Family {
                    self.family.from = matches!(self.screen, Screen::Home | Screen::WalletsStart)
                        .then_some(self.screen);
                }
                self.screen = s;
                self.osk_leave();
                self.list_offset = 0.0;
                self.card_offset = 0.0;
                self.files_out_offset = 0.0;
                self.glance = false;
                self.renaming = None;
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
            | Action::EntryWallet(..)) => self.entry_act(a),
            a @ (Action::PickWallet(..)
            | Action::PickKey(..)
            | Action::RemoveWallet(..)
            | Action::RemoveKeys
            | Action::StartSpend(..)
            | Action::Step(..)
            | Action::StepNext(..)
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
            | Action::Rename
            | Action::TOther(..)
            | Action::CarryToOutbox) => self.wallet_act(a),
            a @ (Action::InboxRemove(..)
            | Action::OutboxRemove(..)
            | Action::InboxLoad
            | Action::InboxChoose(..)
            | Action::PotentialOpen(..)
            | Action::XpubOpen(..)
            | Action::BackupOpen(..)
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
            | Action::IdleLock(..)
            | Action::IdleOff(..)) => self.power_act(a, &c),
            a @ (Action::InputUse(..)
            | Action::InputIgnore(..)
            | Action::Mode(..)
            | Action::ModeSheet
            | Action::DevicesCheck
            | Action::DevicesContinue
            | Action::DevicesUnderstood) => self.devices_act(a),
            Action::PassChip(f) => self.demo_pass_chip(f),
            a @ (Action::Samples
            | Action::SampleTick(..)
            | Action::SamplesAll
            | Action::SamplesLoad
            | Action::UseSample
            | Action::ScanSample) => self.samples_act(a),
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
            a @ (Action::TzStep(..)
            | Action::OpenSetTime
            | Action::SetTimeFocus(..)
            | Action::SetTimeSet
            | Action::SetTimeKeep
            | Action::SetTimeBack) => self.time_act(a),
            a @ (Action::OpenCodes | Action::CodesOpen(..) | Action::CodesAdd) => self.codes_act(a),
            Action::Chart(c) => self.chart_act(c),
            Action::Glance(open) => {
                self.glance = open && self.screen == Screen::Wallets;
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
            | Action::BCodex32(..)
            | Action::BPaperHide
            | Action::BCompact(..)
            | Action::BPin(..)
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
            | Action::BOut(..)) => self.backup_act(a, &c),
            Action::Board(p) => self.board_act(p),
            Action::Catalog(i) => self.catalog_go(usize::from(i)),
            a @ (Action::DecodeFinished
            | Action::DecodeInbox(..)
            | Action::QrDecoded
            | Action::DecodeHex
            | Action::SpendHex) => self.decode_act(a, &c),
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
            // Acted on before any other press (`act_inner`).
            Action::StickPill(_) => {}
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
        if let Some(record) = w.policy().record() {
            return self
                .session
                .keys
                .all()
                .enumerate()
                .filter(|(_, k)| {
                    k.has_words()
                        && k.share
                            .is_some_and(|p| record.info.pubshares.contains(&Some(p)))
                })
                .map(|(i, _)| i)
                .collect();
        }
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
            .filter(|(_, k)| k.has_words() && fps.contains(&k.fingerprint))
            .map(|(i, _)| i)
            .collect()
    }

    /// Costs what `vault_has_wallet`, `vault_key` and `seed_copy_checked`
    /// cost. Where the wallet being backed up and each of its keys are
    /// kept, as the small panel's copy page lists them: in which open
    /// vault, whether the copy by hand was checked, whether a file of it is
    /// in the Outbox unprotected, or not here at all.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub fn backup_kept(&self, c: &Compute) -> Option<wallet_backup::Kept> {
        use crate::vault::screen::{vault_has_wallet, vault_key};
        use wallet_backup::{KeptSeed, Tone};
        let b = self.backup.as_ref()?;
        let wallet = self.session.wallets.get(b.wallet)?;
        let open = 0..self.vaults.open.len();
        // A locked vault cannot be looked into.
        let unknown = self.vaults.open.is_empty() && !self.vault_files().is_empty();
        let not_known = || (EN.backup_kept_not_checked.to_string(), Tone::Dim);
        let wallet_line = match open.clone().find(|&v| vault_has_wallet(c, self, v, wallet)) {
            Some(v) => (
                fill(
                    EN.backup_kept_wallet_in,
                    &[&self.vaults.open[v].name.to_string()],
                ),
                Tone::Ok,
            ),
            None if unknown => not_known(),
            None => (EN.backup_kept_wallet.to_string(), Tone::Warn),
        };
        let stem = file_stem(&wallet.name);
        let here = self.backup_keys(b.wallet);
        let seed_lines = |key: &faraday_keys::KeyInfo| -> Vec<(String, Tone)> {
            let fp = key.fingerprint;
            let mut lines = Vec::new();
            lines.push(
                match open
                    .clone()
                    .find_map(|v| vault_key(c, self, v, key).map(|p| (v, p)))
                {
                    Some((v, with)) => (
                        fill(
                            EN.backup_kept_in,
                            &[
                                &self.vaults.open[v].name.to_string(),
                                &(if with {
                                    format!(" {}", EN.backup_with_its_passphrase)
                                } else {
                                    String::new()
                                }),
                            ],
                        ),
                        Tone::Ok,
                    ),
                    None if unknown => not_known(),
                    None => (EN.backup_kept_not_in.to_string(), Tone::Warn),
                },
            );
            lines.push(if self.seed_copy_checked(c, fp.0) {
                (EN.backup_kept_paper_copy.to_string(), Tone::Ok)
            } else {
                (EN.backup_kept_paper_not.to_string(), Tone::Warn)
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
                    fill(EN.backup_kept_file_unprotected, &[self.medium.for_the()]),
                    Tone::Err,
                ));
            }
            lines
        };
        let away = || vec![(EN.backup_kept_not_here.to_string(), Tone::Dim)];
        let mut seeds = Vec::new();
        if let Some(record) = wallet.policy().record() {
            // A threshold wallet's keys are its shares, by number.
            for (id, pubshare) in record.info.pubshares.iter().enumerate() {
                let key = here
                    .iter()
                    .map(|&k| &self.session.keys[k])
                    .find(|k| pubshare.is_some() && k.share.as_ref() == pubshare.as_ref());
                seeds.push(match key {
                    Some(key) => KeptSeed {
                        name: fp_text(key.fingerprint),
                        lines: seed_lines(key),
                    },
                    None => KeptSeed {
                        name: fill(EN.backup_kept_share, &[&(id + 1).to_string()]),
                        lines: away(),
                    },
                });
            }
        } else {
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
            for fp in fps {
                let key = here
                    .iter()
                    .map(|&k| &self.session.keys[k])
                    .find(|k| k.fingerprint == fp);
                seeds.push(KeptSeed {
                    name: fp_text(fp),
                    lines: key.map_or_else(away, seed_lines),
                });
            }
        }
        Some(wallet_backup::Kept {
            wallet: wallet_line,
            seeds,
        })
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
        if let Some(record) = wallet.policy().record() {
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
                        Some(k) => (fp_text(self.session.keys[k].fingerprint), Some(k)),
                        None => (
                            fill(EN.backup_seed_list_of_share, &[&(id + 1).to_string()]),
                            None,
                        ),
                    }
                })
                .collect();
        }
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

    /// Costs a parse the first time. Wallet description `text` read
    /// ([`wallet::read_wallet`]): the first time it is asked, by whoever
    /// asks, a frame included, and kept on the state by its text, a lookup
    /// after. For a description no Inbox file or vault record keeps read:
    /// split sheets put together, a threshold wallet's descriptor.
    pub(crate) fn read_kept(
        &self,
        c: &Compute,
        text: &str,
    ) -> Option<faraday_bip::policy::WalletPolicy> {
        if let Some((_, p)) = self.texts_read.borrow().iter().find(|(t, _)| t == text) {
            return p.clone();
        }
        let p = wallet::read_wallet(c, text).ok();
        let mut kept = self.texts_read.borrow_mut();
        if kept.len() >= TEXTS_READ_KEPT {
            kept.remove(0);
        }
        kept.push((text.to_string(), p.clone()));
        p
    }

    /// Costs a parse of the config the first time: the name multisig
    /// config `text` carries on its `Name:` line
    /// (`faraday_bip::multisig_config::parse_named`), kept on the state by
    /// its text ([`Faraday::config_names`]), a lookup after.
    pub(crate) fn config_name_kept(&self, c: &Compute, text: &str) -> Option<String> {
        if let Some((_, n)) = self.config_names.borrow().iter().find(|(t, _)| t == text) {
            return n.clone();
        }
        let n = {
            faraday_bip::multisig_config::parse_named(c, text)
                .ok()
                .and_then(|(_, n)| n)
        };
        let mut kept = self.config_names.borrow_mut();
        if kept.len() >= CONFIG_NAMES_KEPT {
            kept.remove(0);
        }
        kept.push((text.to_string(), n.clone()));
        n
    }

    /// Costs a parse of the description the first time: description
    /// `text` parsed as Create builds a wallet ([`create::build`]'s
    /// reader, `WalletPolicy::parse_any`), with the parser's refusal, kept
    /// on the state by its text ([`Faraday::texts_parsed`]), a lookup
    /// after.
    pub(crate) fn parsed_kept(
        &self,
        c: &Compute,
        text: &str,
    ) -> Result<faraday_bip::policy::WalletPolicy, String> {
        if let Some((_, p)) = self.texts_parsed.borrow().iter().find(|(t, _)| t == text) {
            return p.clone();
        }
        let p =
            { faraday_bip::policy::WalletPolicy::parse_any(c, text).map_err(|e| e.to_string()) };
        let mut kept = self.texts_parsed.borrow_mut();
        if kept.len() >= TEXTS_PARSED_KEPT {
            let _ = kept.remove(0);
        }
        kept.push((text.to_string(), p.clone()));
        p
    }

    /// Costs an address derivation per loaded wallet and address tried,
    /// up to its first hundred on each chain, and a signature check, the
    /// first time: signed message `text` checked
    /// ([`wallet::Session::check_message`]), kept by its text, the network
    /// and the loaded wallets ([`Faraday::message_checked`]), a lookup
    /// after.
    pub(crate) fn message_check(
        &self,
        c: &Compute,
        text: &str,
    ) -> Option<std::rc::Rc<wallet::MessageCheck>> {
        let network = self.session.network();
        let sums: Vec<String> = self
            .session
            .wallets
            .iter()
            .map(|w| w.checksum().to_string())
            .collect();
        if let Some(k) = self.message_checked.borrow().as_ref()
            && k.text == text
            && k.network == network
            && k.wallets == sums
        {
            return k.check.clone();
        }
        let check = self.session.check_message(c, text).map(std::rc::Rc::new);
        *self.message_checked.borrow_mut() = Some(KeptMessage {
            text: text.to_string(),
            network,
            wallets: sums,
            check: check.clone(),
        });
        check
    }

    /// Costs a map of the plan and its check the first time. The plan of
    /// `shape` under answers `a`, mapped with its check, for every helper
    /// that reads the plan's map (`docs/SPEED.md` §3.3): mapped the first
    /// time these shape and answers are asked, by whoever asks, a frame
    /// included, and kept on the state by them ([`Faraday::plans_mapped`]),
    /// a lookup after. A frame and a press read the same map, and it is the
    /// map of the state as it is: a change to the answers or the shape is
    /// another key.
    pub(crate) fn plan_mapped(
        &self,
        c: &Compute,
        shape: &plan::Shape,
        a: &plan::Answers,
    ) -> std::rc::Rc<plan::Mapped> {
        if let Some((_, _, m)) = self
            .plans_mapped
            .borrow()
            .iter()
            .find(|(s, x, _)| s == shape && x == a)
        {
            return m.clone();
        }
        let m = std::rc::Rc::new({
            let m = plan::Mapped::of(c, shape, a);
            m.check(c, shape, a);
            m
        });
        let mut kept = self.plans_mapped.borrow_mut();
        // The plans a session shows at once: the backup's, a flow's, a
        // chart's; an answer changed makes another, and the oldest goes.
        if kept.len() >= PLANS_MAPPED_KEPT {
            kept.remove(0);
        }
        kept.push((shape.clone(), a.clone(), m.clone()));
        m
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
            .and_then(|i| self.backup_flows[i].wallet.as_ref())
            .map_or(0, |w| w.shape.vaults);
        plan::Shape {
            m: m.clamp(1, seeds.len().max(1)),
            keys: n.max(1),
            seeds,
            splits: wallet_backup::splits(wallet),
            vaults,
        }
    }

    /// Costs what `plan_mapped` costs. The cards of the backup's plan, in
    /// order (`docs/BACKUP-FLOW.md` §15.1): the board, the software where
    /// the description goes into software or as files, and The plan.
    pub fn backup_questions(&self, c: &Compute) -> Vec<u8> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        // The board replaces the cards from Places to Where the
        // description goes (`docs/BACKUP-FLOW.md` §17.1); the software and
        // The plan stay. Start from is gone (§19.21 item 1).
        let shape = self.plan_shape_now(b.wallet);
        plan_cards_on(&self.plan_mapped(c, &shape, &b.answers))
    }

    /// Costs what `plan_mapped` costs. The backup's checklist, by
    /// [`bstep`]: only what the plan needs. Works out the plan's shape
    /// (kept for the frame while one is drawn).
    ///
    /// Works out [`crate::drawing::View::backup_items`] from the state as it is.
    pub fn backup_items_now(&self, c: &Compute) -> Vec<u8> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let shape = self.plan_shape_now(b.wallet);
        plan::checklist_on(&shape, &b.answers, &self.plan_mapped(c, &shape, &b.answers))
            .into_iter()
            .map(bstep::of)
            .collect()
    }

    /// Opens checklist item `n`, or closes them all. A seed's copy shows
    /// that seed.
    fn backup_item_open(&mut self, n: Option<u8>) {
        let shape = self.backup.as_ref().map(|b| self.plan_shape_now(b.wallet));
        let key = n
            .and_then(bstep::item)
            .and_then(|it| match it {
                plan::Item::Copy(i) | plan::Item::Passphrase(i) => Some(i),
                // A vault's item shows its first seed.
                plan::Item::Vault(v) => {
                    let b = self.backup.as_ref()?;
                    b.answers.vault_seeds(shape.as_ref()?, v).first().copied()
                }
                _ => None,
            })
            .and_then(|i| {
                let w = self.backup.as_ref()?.wallet;
                self.backup_seed_list_now(w).get(i)?.1
            });
        // More options opens closed, unless a file under it is already
        // For the stick.
        let more = n == Some(bstep::PUBLIC)
            && self.backup.as_ref().is_some_and(|b| {
                self.session.wallets.get(b.wallet).is_some_and(|w| {
                    wallet_backup::public_more_files(w)
                        .iter()
                        .any(|(_, f)| self.outbox.iter().any(|i| i.name == *f))
                })
            });
        if let Some(b) = self.backup.as_mut() {
            // A passphrase item opens, and closes, with it hidden.
            if n.is_some_and(bstep::is_passphrase) || b.open.is_some_and(bstep::is_passphrase) {
                b.reveal = false;
            }
            b.open = n;
            b.public_more = more;
            b.scroll.follow = true;
            b.check = None;
            if let Some(k) = key
                && b.key != k
            {
                b.key = k;
                b.reveal = false;
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

    /// Costs what `backup_public_names_for_now` costs. The files
    /// [`Faraday::backup_public`] makes, in the form chosen: the text, the
    /// labelled picture, or both; a file with no picture as its text.
    fn backup_public_names(&self, c: &Compute) -> Vec<String> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        self.backup_public_names_for_now(c, b.wallet, &b.answers)
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

    /// Costs what `copy_checked`, `backup_vault_fits`,
    /// `backup_public_names` and the rest it calls cost. Whether a
    /// checklist item is done, by what it does (it names files and may read
    /// the outbox, so kept for the frame while one is drawn): its file For
    /// the stick or written, its seeds or the wallet in the open vault, the
    /// copy matched, the descriptor shown; the envelopes alone by a press.
    ///
    /// Works out [`crate::drawing::View::backup_item_done`] from the state as it is.
    pub fn backup_item_done_now(&self, c: &Compute, item: plan::Item) -> bool {
        let Some(b) = self.backup.as_ref() else {
            return false;
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return false;
        };
        let stem = file_stem(&w.name);
        // For the stick, or written by the last visit, or by the flow.
        let out = |name: &str| {
            self.outbox.iter().any(|i| i.name == name)
                || self.receipt.as_ref().is_some_and(|r| r.wrote(name))
                || self.wallet_wrote(name)
        };
        let seeds = self.backup_seed_list_now(b.wallet);
        let here: Vec<(usize, &faraday_keys::KeyInfo)> = seeds
            .iter()
            .enumerate()
            .filter_map(|(i, (_, k))| Some((i, self.session.keys.info((*k)?)?)))
            .collect();
        match item {
            // Printed from Faraday Starter: no file is made here.
            plan::Item::Templates => b.printed,
            plan::Item::Copy(i) => here
                .iter()
                .find(|(j, _)| *j == i)
                .is_some_and(|(_, k)| self.copy_checked(c, b, i, k)),
            plan::Item::Passphrase(i) => {
                here.iter().find(|(j, _)| *j == i).is_some_and(|(_, k)| {
                    b.checked
                        .contains(&(k.fingerprint.0, plan::CopyForm::Passphrase))
                })
            }
            plan::Item::Vault(v) => self.backup_vault_fits(c, v).is_some(),
            plan::Item::SeedFiles => {
                !here.is_empty() && here.iter().all(|(_, k)| self.seed_file_made(&stem, k))
            }
            plan::Item::Sheets => {
                if b.answers.split && wallet_backup::splits(w) {
                    let (_, n) = Session::quorum(w);
                    out(&format!("{stem}-share-1-of-{n}.pdf"))
                } else {
                    out(&format!("{stem}-backup.pdf"))
                }
            }
            plan::Item::PublicFiles => self.backup_public_names(c).iter().all(|n| out(n)),
            plan::Item::ShowDescriptor => b.shown,
            plan::Item::Envelopes => b.envelopes,
            // Every stick of the plan written, inside the flow.
            plan::Item::Sticks => self.wallet_sticks_done(c),
        }
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
                    || self.receipt.as_ref().is_some_and(|r| r.wrote(&name))
                    || self.wallet_wrote(&name)
            })
    }

    /// The fingerprints of the seeds the plan puts into vault `v`, and of
    /// those it puts into another vault and not this one.
    pub(crate) fn backup_vault_keys(
        &self,
        v: usize,
    ) -> (
        Vec<faraday_bip::keys::Fingerprint>,
        Vec<faraday_bip::keys::Fingerprint>,
    ) {
        let Some(b) = self.backup.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let shape = self.plan_shape_now(b.wallet);
        let list = self.backup_seed_list_now(b.wallet);
        let fp = |i: usize| {
            let k = list.get(i)?.1?;
            Some(self.session.keys.info(k)?.fingerprint)
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
        let shape = self.plan_shape_now(b.wallet);
        let list = self.backup_seed_list_now(b.wallet);
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
            Some(name) if !name.is_empty() => fill(EN.plan_vault_2, &[&n, name]),
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
        (0..self.backup_flows[i].plan.vaults.len())
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

    /// Costs what `backup_vault_shares` and `backup_vault_passphrases`
    /// cost. Whether the plan puts anything in vault `v`.
    fn backup_vault_wanted(&self, c: &Compute, v: usize) -> Option<()> {
        let b = self.backup.as_ref()?;
        self.session.wallets.get(b.wallet)?;
        let (own_seeds, _) = self.backup_vault_seeds(v);
        let shares = !self.backup_vault_shares(c, v).is_empty();
        let passphrases = !self.backup_vault_passphrases(c, v).is_empty();
        (!own_seeds.is_empty() || b.answers.wallet_in_vault(v) || shares || passphrases)
            .then_some(())
    }

    /// Costs what `plan_mapped` costs. The session keys whose passphrase
    /// the plan keeps alone in vault `v`, as a passphrase record
    /// (`docs/BACKUP-FLOW.md` §17.12 item 4): a key with a passphrase only.
    pub(crate) fn backup_vault_passphrases(&self, c: &Compute, v: usize) -> Vec<usize> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let shape = self.plan_shape_now(b.wallet);
        let list = self.backup_seed_list_now(b.wallet);
        self.plan_mapped(c, &shape, &b.answers)
            .vault_passphrases(v)
            .into_iter()
            .filter_map(|i| list.get(i)?.1)
            .filter(|&k| {
                self.session
                    .keys
                    .info(k)
                    .is_some_and(|key| key.has_passphrase)
            })
            .collect()
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

    /// Costs what `plan_mapped` costs. The shares the plan keeps in vault
    /// `v`, each with its text and how many shares there are
    /// (`docs/BACKUP-FLOW.md` §15.6).
    pub(crate) fn backup_vault_shares(&self, c: &Compute, v: usize) -> Vec<(usize, String, usize)> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return Vec::new();
        };
        let shape = self.plan_shape_now(b.wallet);
        let (m, n) = Session::quorum(w);
        let plan = wallet_backup::split_plan(n, m, b.answers.omit);
        self.plan_mapped(c, &shape, &b.answers)
            .vault_shares(v)
            .into_iter()
            .filter_map(|j| Some((j, wallet_backup::split_share(w, plan.get(j)?)?, plan.len())))
            .collect()
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

    /// Costs what `backup_vault_wanted`, `vault_holds_seed`,
    /// `vault_has_wallet` and the rest it calls cost. Whether open vault
    /// `o` holds what the plan puts in vault `v` and no seed only another
    /// vault is to hold.
    pub(crate) fn backup_open_fits(&self, c: &Compute, v: usize, o: usize) -> bool {
        use crate::vault::screen::vault_has_wallet;
        let Some(b) = self.backup.as_ref() else {
            return false;
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return false;
        };
        if self.backup_vault_wanted(c, v).is_none() {
            return false;
        }
        let (own_seeds, other_seeds) = self.backup_vault_seeds(v);
        let wallet_too = b.answers.wallet_in_vault(v);
        own_seeds
            .iter()
            .all(|&(k, with)| self.vault_holds_seed(c, o, k, with))
            && !other_seeds
                .iter()
                .any(|&k| self.vault_holds_seed(c, o, k, false))
            && (!wallet_too || vault_has_wallet(c, self, o, w))
            && self
                .backup_vault_shares(c, v)
                .iter()
                .all(|(j, text, _)| self.vault_holds_share(c, o, w.checksum(), *j, text))
            && self
                .backup_vault_passphrases(c, v)
                .iter()
                .all(|&k| self.vault_holds_passphrase(o, k))
    }

    /// Costs what `backup_vault_shares`, `backup_vault_passphrases` and
    /// `backup_vault_open` cost. The vault that does the plan's vault `v`:
    /// an open vault, or one locked since this power-on by what it was seen
    /// to hold, that holds every seed the plan puts into `v` (with its
    /// passphrase where the plan puts it there), the wallet where the plan
    /// puts it there, and no seed only another vault is to hold. Its name.
    pub(crate) fn backup_vault_fits(&self, c: &Compute, v: usize) -> Option<String> {
        let b = self.backup.as_ref()?;
        let w = self.session.wallets.get(b.wallet)?;
        let wallet_too = b.answers.wallet_in_vault(v);
        let (own_seeds, _) = self.backup_vault_seeds(v);
        let shares = !self.backup_vault_shares(c, v).is_empty();
        let passphrases = !self.backup_vault_passphrases(c, v).is_empty();
        if own_seeds.is_empty() && !wallet_too && !shares && !passphrases {
            return None;
        }
        if let Some(o) = self.backup_vault_open(c, v) {
            return Some(self.vaults.open[o].name.clone());
        }
        // A locked vault's summary lists no shares and no passphrases: a
        // vault that is to hold one is done only while it is open.
        if shares || passphrases {
            return None;
        }
        let (own, others) = self.backup_vault_keys(v);
        let sum = w.checksum().to_string();
        // Each seed told by its whole master id, the session key's else
        // the backup record's (F65); one that cannot be told is not done.
        let record = self.backup_records.iter().find(|r| r.sum == sum);
        let has = |s: &crate::vault::VaultSummary, f: &faraday_bip::keys::Fingerprint| {
            crate::backups::summary_has_seed(s, f.0, self.seed_id(record, f.0))
        };
        let salts: Vec<[u8; 32]> = self.vaults.open.iter().map(|o| o.header().salt).collect();
        self.vaults
            .summaries
            .iter()
            .filter(|s| !salts.contains(&s.salt))
            .find(|s| {
                own.iter().all(|f| has(s, f) == Some(true))
                    && others.iter().all(|f| has(s, f) == Some(false))
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
            .map_or_else(
                || fill(EN.place_name_place, &[&(p + 1).to_string()]),
                str::to_string,
            )
    }

    /// The text a vault keeps a wallet under, as Vaults saves it.
    pub(crate) fn wallet_text(w: &wallet::Wallet) -> String {
        match (w.policy().record(), w.policy().silent()) {
            (Some(r), _) => r.to_text(),
            (None, Some(s)) => s.to_text(),
            (None, None) => w.policy().to_descriptor_checksummed(),
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
        let Some(b) = self.backup.as_ref() else {
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
            let others: Vec<usize> = (0..self.backup_flows[i].plan.vaults.len())
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
        let want = wallet::same_wallet(w.policy());
        // This wallet's plan records, by place.
        let ours: Vec<usize> = self
            .vaults
            .described_of(&c, open.id, kind::PLAN)
            .filter(|(_, _, d)| d.same == want)
            .map(|(i, _, _)| i)
            .collect();
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
    }

    fn backup_out(&mut self, what: u8) {
        let Some(wallet) = self.backup.as_ref().map(|b| b.wallet) else {
            return;
        };
        self.public_out(wallet, what);
    }

    /// Costs a QR encoding (through `picture_codes`). The labelled pictures
    /// among a wallet's public files `what` (as [`Faraday::public_out`]
    /// numbers them): for 8, the descriptor's code, in parts past one code;
    /// for 4, each split share's code, one beside each share's sheet and
    /// text. None for any other file.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub fn public_pictures(&self, c: &Compute, wallet: usize, what: u8) -> Vec<picture::Labelled> {
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
                    .as_ref()
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
                    lines.push(EN.public_pictures_not.to_string());
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
            _ => Vec::new(),
        }
    }

    /// One of a wallet's public files to the Outbox: 0 the blank
    /// template, 1 the descriptor, 2 the multisig config, 3 the backup
    /// sheet, 4 the split shares, 5 the wallet .json, 6 the BIP 129
    /// descriptor record, 7 Bitcoin Core's `importdescriptors` file, 8 the
    /// descriptor's code as a labelled picture; under More options
    /// ([`wallet_backup::public_more_files`]), 9 Bitcoin Core's import for Core before
    /// 29, 10 the Electrum wallet file, 11 the Caravan config, 13 the
    /// Wasabi file (`docs/INTEROP.md` §5 I4; 12 is Specter DIY's, I8).
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
            k => self.toast_out(&fill(EN.public_out_files, &[&k.to_string()])),
        }
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

    /// The backup under way's wallet, when its checklist is showing and
    /// file `what` is For the stick: that file is made again when what it
    /// depends on changes. The old one goes now.
    fn backup_remake(&mut self, what: u8) -> Option<usize> {
        let b = self.backup.as_ref()?;
        if b.stage != BStage::Checklist {
            return None;
        }
        let w = b.wallet;
        let made = self.public_names(w, what);
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
        let c = self.compute();
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
            .is_some_and(|wl| b.answers.split && wallet_backup::splits(wl));
        let items: Vec<plan::Item> = self
            .backup_items_now(&c)
            .into_iter()
            .filter_map(bstep::item)
            .collect();
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        for item in items {
            match item {
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
                                self.code_view(&c, code)
                                    .ok()
                                    .and_then(|v| v.png(&c))
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
                    // Each key's account key files go with them.
                    files.extend(self.account_key_files(&c, w));
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
            k => self.toast_out(&fill(EN.backup_files_make_files, &[&k.to_string()])),
        }
    }

    /// The files [`Faraday::public_out`] makes, with their bytes.
    fn public_made(&mut self, wallet: usize, what: u8) -> Vec<(String, Vec<u8>)> {
        self.public_files(wallet, what, true)
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
        let Some(w) = self.session.wallets.get(wallet) else {
            return Vec::new();
        };
        let omit = self
            .backup
            .as_ref()
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
                // A threshold wallet signs only with its record, which
                // lists every public share.
                if let Some(r) = w.policy().record() {
                    files.push((
                        format!("{stem}-record.txt"),
                        format!("{}\n", r.to_text()).into_bytes(),
                    ));
                }
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
            13 => {
                if let Some(c) = faraday_intake::single_key::wasabi_file(w.policy()) {
                    files.push((format!("{stem}-wasabi.json"), c.into_bytes()));
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

    /// Costs what `account_text` costs. The account key in slot `slot` of
    /// wallet `i`, when this session holds its seed: for the cosigners of a
    /// wallet of more than one key. `None` for a threshold or silent
    /// payments wallet, whose slots are not account keys.
    pub fn wallet_key(&self, c: &Compute, i: usize, slot: u8) -> Result<WalletKey, String> {
        let w = self
            .session
            .wallets
            .get(i)
            .ok_or(EN.wallet_key_that_wallet)?;
        if w.policy().record().is_some() || w.policy().silent().is_some() {
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
    /// wallet is no threshold or silent payments wallet. A lookup; it
    /// derives nothing.
    fn wallet_key_here(&self, i: usize, slot: u8) -> Option<faraday_bip::keys::Fingerprint> {
        let w = self.session.wallets.get(i)?;
        if w.policy().record().is_some() || w.policy().silent().is_some() {
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

    /// Costs what `account_text` costs. The account key expression for a
    /// slot held here, with its fingerprint, for the cosigners.
    fn create_key_text(&self, c: &Compute, slot: u8) -> Result<(String, String), String> {
        let cs = self.create.as_ref().ok_or(EN.create_key_text_nothing)?;
        let Some(create::Source::Here(fp)) = cs.slots.get(slot as usize) else {
            return Err(EN.create_key_text_that.into());
        };
        let key = self
            .session
            .keys
            .position_of(*fp)
            .ok_or(EN.create_key_text_a_chosen)?;
        Ok((
            fp_text(faraday_bip::keys::Fingerprint(*fp)),
            self.session
                .keys
                .account_text(c, key, cs.kind, cs.account)?,
        ))
    }

    /// Costs a key derivation (through `account_ur_at`). A slot's key as
    /// `ur:crypto-account`, for a `wsh` multisig, the one script that type
    /// writes a cosigner's key under.
    fn create_key_account(&self, c: &Compute, slot: u8) -> Option<String> {
        let cs = self.create.as_ref()?;
        let Some(create::Source::Here(fp)) = cs.slots.get(slot as usize) else {
            return None;
        };
        let key = self.session.keys.position_of(*fp)?;
        cs.kind
            .account_ur_at(c, &self.session.keys, key, cs.account)
    }

    /// Costs a key derivation (through `bsms_record`). A slot's key as a
    /// signed BIP 129 key record, with its fingerprint: for a `wsh` or
    /// `sh(wsh)` multisig only.
    fn create_key_bsms(&self, c: &Compute, slot: u8) -> Option<(String, String)> {
        let cs = self.create.as_ref()?;
        let Some(create::Source::Here(fp)) = cs.slots.get(slot as usize) else {
            return None;
        };
        let key = self.session.keys.position_of(*fp)?;
        let fpt = fp_text(faraday_bip::keys::Fingerprint(*fp));
        cs.kind
            .bsms_record(c, &self.session.keys, key, cs.account, &fpt)
            .map(|r| (fpt, r))
    }

    /// Costs what `account_text` costs. The key texts of a creation's
    /// slots, or why they are not ready.
    pub fn create_keys(&self, c: &Compute) -> Result<Vec<String>, String> {
        let cs = self.create.as_ref().ok_or(EN.create_keys_nothing)?;
        if cs.kind.threshold() {
            return self.create_shares().map(|keys| {
                keys.iter()
                    .map(|&k| fp_text(self.session.keys[k].fingerprint))
                    .collect()
            });
        }
        let mut out = Vec::new();
        for (i, s) in cs.slots.iter().enumerate() {
            match s {
                create::Source::Empty => {
                    return Err(fill(EN.create_keys_key_has, &[&(i + 1).to_string()]));
                }
                create::Source::Later => {
                    return Err(fill(
                        EN.create_keys_key_waits,
                        &[&(i + 1).to_string(), self.medium.a(), &(i + 1).to_string()],
                    ));
                }
                create::Source::Here(fp) => {
                    let key = self
                        .session
                        .keys
                        .position_of(*fp)
                        .ok_or(EN.create_keys_a_chosen)?;
                    out.push(
                        self.session
                            .keys
                            .account_text(c, key, cs.kind, cs.account)?,
                    );
                }
                create::Source::Cosigner(text) => out.push(text.clone()),
            }
        }
        let mut seen = out.clone();
        seen.sort();
        seen.dedup();
        if seen.len() != out.len() {
            return Err(EN.create_keys_the_same.into());
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
    fn create_shares(&self) -> Result<Vec<usize>, String> {
        let c = self.create.as_ref().ok_or(EN.create_shares_nothing)?;
        let mut out: Vec<usize> = Vec::new();
        for (i, s) in c.slots.iter().enumerate() {
            let create::Source::Here(fp) = s else {
                return Err(fill(EN.create_shares_share_has, &[&(i + 1).to_string()]));
            };
            let key = self
                .session
                .keys
                .position_of(*fp)
                .ok_or(EN.create_shares_a_chosen)?;
            if self.session.keys[key].share.is_none() {
                return Err(fill(EN.create_shares_share, &[&(i + 1).to_string()]));
            }
            if out
                .iter()
                .any(|&k| self.session.keys[k].fingerprint.0 == *fp)
            {
                return Err(EN.create_shares_the_same.into());
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
        let c = self.compute();
        // The dealt shares are new seeds: none in the demo.
        if !self.makes_secrets() {
            return;
        }
        let (m, n) = match self.create.as_ref() {
            Some(c) => (c.m, c.n),
            None => return,
        };
        let dealt = self.create_shares().and_then(|keys| {
            self.session
                .keys
                .deal(&c, &keys, n, m)
                .map_err(|e| match e {
                    faraday_keys::frost::DealError::NoShare => EN.create_deal_a_chosen.to_string(),
                    faraday_keys::frost::DealError::Refused(e) => e,
                })
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
        let name = fill(
            EN.create_deal_new_wallet,
            &[&(self.session.wallets.len() + 1).to_string()],
        );
        let i = match self.session.add_wallet(
            &c,
            &name,
            &dealt.record,
            EN.create_deal_dealt_here,
            wallet::Origin::Statement,
        ) {
            Ok(i) => i,
            Err(e) => {
                if let Some(c) = self.create.as_mut() {
                    c.error = Some(e.text());
                }
                return;
            }
        };
        self.session.keys.add_dealt(&c, dealt, |id| {
            fill(
                EN.create_deal_share_of,
                &[&(id + 1).to_string(), &name.to_string()],
            )
        });
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

    /// Why Restore does not take the seed whose key is `fp`, if it does
    /// not: with the description known, not one of its keys; from the
    /// seeds alone, no slot left. Once the wallet is made and Keys is
    /// closed, a key added is no seed of Restore's: nothing to refuse.
    pub(crate) fn restore_refusal(&self, fp: faraday_bip::keys::Fingerprint) -> Option<String> {
        let r = self.restore.as_ref().filter(|r| !r.keys_made())?;
        if r.described {
            let w = self.session.wallets.get(r.wallet?)?;
            // A threshold wallet's slots are shares, matched as they load.
            if w.policy().record().is_some() {
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
            .then(|| EN.restore_refusal_every.to_string())
    }

    /// A seed for Restore's open slot, loaded: matched to the
    /// description's keys or put in the slot (`docs/history/NEW-WALLET.md`
    /// §12.1).
    pub(crate) fn restore_took(&mut self, fp: faraday_bip::keys::Fingerprint) {
        if let Some(why) = self.restore_refusal(fp) {
            if let Some(r) = self.restore.as_mut() {
                r.error = Some(why);
            }
            return;
        }
        if self.restore.as_ref().is_some_and(RestoreState::keys_made) {
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
    }

    /// A description or a share read by the camera while Restore waits
    /// for one: the wallet loads, or the share is taken. Returns whether
    /// it was.
    fn restore_arrived(&mut self, at: usize) -> bool {
        let c = self.compute();
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
                    restore::merge(&self.restore_share_texts(&c)).is_ok_and(|m| m.whole.is_some());
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
            r.error = Some(fill(
                EN.restore_described_make_still,
                &[
                    &open.to_string(),
                    (if open == 1 {
                        EN.restore_described_make_slot
                    } else {
                        EN.restore_described_make_slots
                    }),
                ],
            ));
            return;
        }
        r.next(rstep::SEEDS);
        self.refresh_spend();
    }

    fn restore_from_vault(&mut self, v: usize, r: usize) {
        let c = self.compute();
        use faraday_vault::records::{field, kind};
        let Some(open) = self.vaults.open.get(v) else {
            return;
        };
        let Some(rec) = self.vaults.records(open.id).get(r) else {
            return;
        };
        let (Some(text), name) = (
            rec.public_text(field::WALLET).map(str::to_string),
            rec.public_text(field::WALLET_NAME)
                .unwrap_or(EN.restore_from_vault_wallet)
                .to_string(),
        ) else {
            return;
        };
        let source = open.name.clone();
        self.restore_wallet(&name, &text, &source, wallet::Origin::Vault);
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
        let keep: BTreeSet<usize> = self
            .vaults
            .records(open.id)
            .of(kind::KEY)
            .filter(|(_, k)| {
                vault::screen::key_fingerprint(&c, self, *k).is_some_and(|fp| want.contains(&fp))
            })
            .map(|(i, _)| i)
            .chain([r])
            .collect();
        let all: BTreeSet<usize> = (0..self.vaults.records(open.id).len()).collect();
        let saved = std::mem::replace(
            &mut self.vaults.open[v].skip,
            all.difference(&keep).copied().collect(),
        );
        let (keys, _) = self.vault_load(v, false);
        self.vaults.open[v].skip = saved;
        self.screen = Screen::Restore;
        if keys > 0 {
            self.toast(&fill(
                EN.restore_from_vault_from,
                &[
                    &keys.to_string(),
                    (if keys == 1 {
                        EN.restore_from_vault_key
                    } else {
                        EN.restore_from_vault_keys
                    }),
                    &source.to_string(),
                ],
            ));
        }
    }

    /// Whether the key in a Create slot can take a passphrase there: one
    /// loaded here from BIP-39 words without one, on a wallet that is not
    /// a threshold wallet (`docs/history/NEW-WALLET.md` §3.6).
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
                .all()
                .find(|k| k.fingerprint.0 == *fp)
                .is_some_and(|k| k.has_words() && !k.has_passphrase)
    }

    /// Lock in under a Create slot: adds the key the slot's words and the
    /// passphrase typed make, and puts it in the slot in place of the
    /// first, which stays loaded (`docs/history/NEW-WALLET.md` §3.6).
    fn create_pass_lock(&mut self) {
        let c = self.compute();
        if !self.selftest_passed() {
            return;
        }
        let Some(slot) = self.create.as_ref().and_then(|cs| cs.pass_slot) else {
            return;
        };
        if !self.create_pass_offered(usize::from(slot)) {
            return;
        }
        let Some(cs) = self.create.as_mut() else {
            return;
        };
        if cs.pass.as_str() != cs.pass2.as_str() {
            cs.pass_note = Some(EN.create_pass_lock_the.to_string());
            return;
        }
        // Empty means none: the key stays as it is.
        if cs.pass.as_str().is_empty() {
            cs.close_pass();
            return;
        }
        let Some(create::Source::Here(first)) = cs.slots.get(usize::from(slot)).cloned() else {
            return;
        };
        let mut pass = secret_text::room();
        pass.push_str(cs.pass.as_str());
        let Some(place) = self.session.keys.position_of(first) else {
            return;
        };
        let key = &self.session.keys[place];
        if !key.has_words() {
            return;
        }
        let label = fill(EN.create_pass_lock_passphrase, &[&key.label.to_string()]);
        let (added, new) = match self
            .session
            .keys
            .add_with_passphrase(&c, place, &pass, &label)
        {
            Ok(fp) => (Ok(fp), true),
            // Loaded already with this passphrase: that key is the one.
            Err(wallet::Refusal::Duplicate(_)) => (
                self.session
                    .keys
                    .fingerprint_with(&c, place, &pass)
                    .ok_or_else(|| wallet::Refusal::Words(String::new()).text()),
                false,
            ),
            Err(e) => (Err(e.text()), false),
        };
        // The passphrase exists nowhere else: the key is made here
        // (`docs/history/NEW-WALLET.md` §14.3).
        if new && let Ok(fp) = &added {
            self.mark_made_here(*fp);
        }
        let Some(cs) = self.create.as_mut() else {
            return;
        };
        match added {
            Ok(fp) => {
                if let Some(s) = cs.slots.get_mut(usize::from(slot)) {
                    *s = create::Source::Here(fp.0);
                }
                if !cs.pass_locked.contains(&fp.0) {
                    cs.pass_locked.push(fp.0);
                }
                cs.close_pass();
                self.refresh_spend();
            }
            Err(e) => cs.pass_note = Some(e),
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
        let k = self.compute();
        if self.create.as_ref().is_some_and(|c| c.kind.threshold()) {
            self.create_deal();
            return;
        }
        let keys = match self.create_keys(&k) {
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
        // A wallet made from another is named after it, with the account
        // (`docs/history/NEW-WALLET.md` §16.2).
        let name = match c.from.as_ref() {
            Some(f) => f.name_at(c.kind, c.account),
            None => fill(
                EN.create_make_new_wallet,
                &[&(self.session.wallets.len() + 1).to_string()],
            ),
        };
        let before = self.session.wallets.len();
        match self.session.add_wallet(
            &k,
            &name,
            &descriptor,
            EN.create_make_made_here,
            wallet::Origin::Statement,
        ) {
            Ok(i) => {
                self.chart_work.moved_to(&self.session, i);
                self.wallet = i;
                let path = create::paths_text(self.session.wallets[i].policy());
                let owner = self.create.as_ref().map(|c| c.owner);
                let before_made = owner
                    .and_then(|o| self.wallet_flow_pre_of(o))
                    .map(|f| (f, Box::new(self.backup_flows[f].clone())));
                if let Some(c) = self.create.as_mut() {
                    c.built = Some(i);
                    c.error = None;
                    c.done[cstep::KEYS as usize] = true;
                    c.open = Some(cstep::CHECK);
                    c.scroll.follow = true;
                    c.path = path;
                    c.added = i == before;
                    c.before_made = before_made;
                }
                // The flow that decided where its keys go knows the wallet
                // now.
                if let Some(owner) = owner {
                    self.wallet_flow_attach(i, owner);
                }
            }
            Err(e) => {
                if let Some(c) = self.create.as_mut() {
                    c.error = Some(e.text());
                }
            }
        }
    }

    /// Costs what `read_kept` costs. The texts of the shares chosen on the
    /// restore screen, and those kept in the open vaults, each vault
    /// share's wallet read as a description no record keeps
    /// ([`Faraday::read_kept`]).
    pub fn restore_share_texts(&self, c: &Compute) -> Vec<String> {
        let Some(r) = self.restore.as_ref() else {
            return Vec::new();
        };
        let mut texts: Vec<String> = self
            .inbox
            .iter()
            .filter(|i| r.shares.contains(&i.name))
            .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
            .collect();
        // The shares kept in every open vault (`docs/BACKUP-FLOW.md`
        // §15.6), each where it agrees with the shares in hand.
        use faraday_vault::records::{field, kind};
        // One wallet's shares, told by field 5, never by name: the first
        // that merges with what is in hand.
        let mut wallet: Option<String> = None;
        for o in &self.vaults.open {
            for (_, rec) in self.vaults.records(o.id).of(kind::WALLET_SHARE) {
                let Some(t) = rec.public_text(field::SHARE) else {
                    continue;
                };
                let of = rec
                    .public_text(field::SHARE_WALLET)
                    .and_then(|d| self.read_kept(c, d))
                    .map(|p| p.checksum().to_string());
                if wallet.is_some() && of != wallet {
                    continue;
                }
                if texts.iter().any(|x| x.trim() == t.trim()) {
                    continue;
                }
                let mut trial = texts.clone();
                trial.push(t.to_string());
                if restore::merge(&trial).is_ok() {
                    texts = trial;
                    wallet = of;
                }
            }
        }
        texts
    }

    /// Loads the wallet a description gives and fills Restore's Kind and
    /// Quorum from it: the seeds are matched to its keys next. `origin`
    /// is what `source` is, given by the caller, which already knows.
    fn restore_wallet(&mut self, name: &str, text: &str, source: &str, origin: wallet::Origin) {
        let c = self.compute();
        match self.session.add_wallet(&c, name, text, source, origin) {
            Ok(i) => {
                self.wallet = i;
                let w = &self.session.wallets[i];
                let kind = create::NewKind::of(w.policy());
                let (m, n) = Session::quorum(w);
                // The flow that decided where the seeds go knows the wallet
                // once it is made from them.
                if let Some(owner) = self
                    .restore
                    .as_ref()
                    .filter(|r| r.into_vaults())
                    .map(|r| r.owner)
                {
                    self.wallet_flow_attach(i, owner);
                }
                if let Some(r) = self.restore.as_mut() {
                    r.wallet = Some(i);
                    r.described = true;
                    r.absent.clear();
                    r.error = None;
                    if let Some(s) = r.seeds.as_mut() {
                        s.fix(kind.unwrap_or(s.kind));
                        // A description's keys are matched to the seeds
                        // from none; a wallet made from the seeds keeps
                        // them, its keys.
                        if origin != wallet::Origin::TypedSeeds {
                            s.keys.clear();
                        }
                        s.set_n(n);
                        s.set_m(m);
                    }
                    for k in [rstep::KIND, rstep::QUORUM, rstep::DESCRIPTION] {
                        r.done[k as usize] = true;
                    }
                    r.done[rstep::SEEDS as usize] = false;
                    r.done[rstep::CHECK as usize] = false;
                    // On to the next card not done: the question of vaults
                    // when it is not answered yet, before the seeds
                    // (`docs/history/NEW-WALLET.md` §12.3 item 4).
                    r.next(rstep::DESCRIPTION);
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
        let c = self.compute();
        if !self.online {
            return;
        }
        match pdf::sheet(&c, &String::from_utf8_lossy(bytes)) {
            Ok(pdf) => self.storage_out.push_back(StorageCommand::Print {
                name: fill(EN.make_pdf_pdf, &[&stem(name).to_string()]),
                bytes: pdf,
            }),
            Err(e) => self.toast(&fill(EN.make_pdf_not_printed, &[&e.to_string()])),
        }
    }

    /// One code the camera read: a part of a UR, a PSBT, or text.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn scanned(&mut self, bytes: Vec<u8>) {
        let c = self.compute();
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
        if camera
            && sticks
            && matches!(
                scan.purpose,
                ScanPurpose::Seed | ScanPurpose::KeyPart | ScanPurpose::WalletKey(_)
            )
        {
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
        if let ScanPurpose::WalletKey(w) = scan.purpose {
            match seed_in_code(&c, &bytes) {
                Some(words) => self.wallet_key_scanned(w, &words),
                None => {
                    let text = String::from_utf8_lossy(&bytes).to_string();
                    scan.note = Some(
                        what_instead_of_a_seed(&c, &text)
                            .unwrap_or(EN.scanned_not_a_seedqr)
                            .to_string(),
                    );
                }
            }
            return;
        }
        if scan.purpose == ScanPurpose::Seed {
            // A seed's words, pasted on the desktop (F75) or in a QR,
            // load as the SeedQR's would.
            if std::str::from_utf8(&bytes).is_ok_and(|t| t.contains(' '))
                && let Some(words) = seed_in_code(&c, &bytes)
            {
                drop(bytes);
                self.seed_scanned(&words);
                return;
            }
            let mut bytes = bytes;
            let lang = faraday_bip::bip39::Language::English;
            let digits = bytes.iter().all(u8::is_ascii_digit);
            let read = if digits {
                faraday_keys::words::from_seedqr_digits(&bytes, lang)
            } else {
                faraday_keys::words::from_compact_seedqr(&bytes, lang)
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
                        what_instead_of_a_seed(&c, &text)
                            .unwrap_or(EN.scanned_not_a_seedqr)
                            .to_string(),
                    );
                }
            }
            return;
        }
        // What do you have?: a seed asks how it loads, a share of a key
        // goes to Add a key's form for it, anything else goes to Files
        // below and the camera stays on.
        let intake = scan.purpose == ScanPurpose::Intake;
        if intake {
            if let Some(words) = seed_in_code(&c, &bytes) {
                if sticks {
                    scan.note = Some(stick_keys(medium));
                    return;
                }
                self.family_intake_seed(&words);
                return;
            }
            if self.intake_key_part(&bytes) {
                return;
            }
        }
        // Home's Scan takes a seed too: a SeedQR, a CompactSeedQR or the
        // words go to Add a key, as its own Scan does, and never into a
        // box (`docs/QR.md` §2).
        if camera && let Some(words) = seed_in_code(&c, &bytes) {
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
        let item = Item::new(&c, &name, data);
        if item.kind == FileKind::Other && !enveloped {
            if let Some(scan) = self.scan.as_mut() {
                scan.note = Some(EN.scanned_not_a_psbt.to_string());
                scan.decoder = faraday_codec::ur::Decoder::new();
            }
            return;
        }
        let is_key = item.kind == FileKind::Key;
        self.inbox.push(item);
        self.save_boxes();
        if intake {
            self.family_intake_file(self.inbox.len() - 1);
            return;
        }
        self.scan = None;
        self.sheet = None;
        self.commands.push_back(Command::CameraOff);
        // A cosigner's account key scanned while a wallet is made from
        // seeds fills the box waiting for one.
        if is_key && self.seeds_take_file(self.inbox.len() - 1) {
            self.toast(&fill(EN.scanned_is_a_cosigner, &[&name.to_string()]));
            return;
        }
        // A cosigner's key scanned while a wallet is being made fills the
        // slot waiting for one.
        if is_key && let Some(slot) = self.create_waiting_slot() {
            let at = self.inbox.len() - 1;
            self.act(Action::CSlotFile(slot, at));
            self.toast(&fill(
                EN.scanned_is_key,
                &[&name.to_string(), &(slot + 1).to_string()],
            ));
            self.screen = Screen::Create;
            return;
        }
        if self.screen == Screen::Family && self.family_arrived(self.inbox.len() - 1) {
            return;
        }
        if self.screen == Screen::Restore && self.restore_arrived(self.inbox.len() - 1) {
            return;
        }
        self.toast(&fill(EN.scanned_is_in_files, &[&name.to_string()]));
        if !matches!(
            self.screen,
            Screen::Spend | Screen::Visit | Screen::Restore | Screen::Home | Screen::Family
        ) {
            self.screen = Screen::Files;
        }
    }

    /// A SLIP-39 share or a codex32 string read by the intake scanner:
    /// Add a key opens on its form with it in, and returns to the Spend
    /// tab once the key is put together. Returns whether it was one.
    fn intake_key_part(&mut self, bytes: &[u8]) -> bool {
        let text = zeroize::Zeroizing::new(String::from_utf8_lossy(bytes).trim().to_lowercase());
        let form = if faraday_bip::slip39::Share::parse(&text).is_ok() {
            forms::Form::Slip39
        } else if faraday_bip::codex32::Codex32::parse(&text).is_ok() {
            forms::Form::Codex32
        } else {
            return false;
        };
        if !self.may_load_keys() {
            if let Some(scan) = self.scan.as_mut() {
                scan.note = Some(stick_keys(self.medium));
            }
            return true;
        }
        self.scan = None;
        self.sheet = None;
        self.commands.push_back(Command::CameraOff);
        self.act(Action::Entry(None));
        if let Some(i) = forms::Form::ALL.iter().position(|f| *f == form) {
            self.act(Action::EntryForm(i as u8));
        }
        self.entry.typed.set(&text);
        self.form_add_part();
        true
    }

    /// A seed's words read by the camera: the camera closes and the
    /// seed waits on its sheet for how it loads (`seed_read`).
    fn seed_scanned(&mut self, words: &str) {
        // A scan fills the words from a camera frame, which keeps the
        // status line's probe: it is worked out again for these.
        self.entry.forget_probe();
        // New key's Scan a SeedQR: its Key card takes the words and asks
        // the passphrase there (`docs/history/NEW-WALLET.md` §15.2).
        if self.keygen_scanned(words) {
            return;
        }
        self.seed_read_open(words);
    }

    /// A code read while the backup's copy item scans the copy of the
    /// seed `fp`: decoded as a SeedQR and compared word by word with that
    /// seed, never loaded. A match closes the camera; a word that differs
    /// is named and the camera stays open for the copy fixed.
    fn copy_scanned(
        &mut self,
        fp: faraday_bip::keys::Fingerprint,
        bytes: zeroize::Zeroizing<Vec<u8>>,
    ) {
        let c = self.compute();
        let copy = copy_indices(&bytes);
        drop(bytes);
        let Some(copy) = copy else {
            if let Some(scan) = self.scan.as_mut() {
                scan.note = Some(EN.copy_scanned_not_a.to_string());
            }
            return;
        };
        let found = self
            .session
            .keys
            .position(fp)
            .and_then(|k| self.session.keys.words_match(&c, k, &copy));
        let Some(found) = found else {
            // The seed is no longer here: nothing to compare with.
            self.scan = None;
            self.sheet = None;
            self.commands.push_back(Command::CameraOff);
            return;
        };
        drop(copy);
        let matched = found == wallet_backup::CopyCheck::Matches;
        if matched {
            self.copy_matched(fp.0, plan::CopyForm::SeedQr);
        }
        if let Some(b) = self.backup.as_mut() {
            b.scanned = Some(found.clone());
        }
        if matched {
            self.scan = None;
            self.sheet = None;
            self.commands.push_back(Command::CameraOff);
        } else if let Some(scan) = self.scan.as_mut() {
            scan.note = Some(wallet_backup::copy_scan_line(&found));
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
        let c = self.compute();
        let scan = self.scan.as_mut()?;
        scan.reads += 1;
        let text = zeroize::Zeroizing::new(String::from_utf8_lossy(&bytes).trim().to_string());
        // A SeedQR is digits only; a seed never arrives by this door.
        if !text.is_empty()
            && text.bytes().all(|b| b.is_ascii_digit())
            && (text.len() == 48 || text.len() == 96)
        {
            scan.note = Some(EN.read_code_that_is_a.to_string());
            return None;
        }
        // A CompactSeedQR is 16 or 32 bytes of entropy, not text.
        if matches!(bytes.len(), 16 | 32) && std::str::from_utf8(&bytes).is_err() {
            scan.note = Some(EN.read_code_that_may.to_string());
            return None;
        }
        // A wrong arrival is named, not refused generically (`docs/QR.md`
        // §1): a seed's words or a private key never go in a box.
        if let Some(why) = secret_text(&c, &text) {
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
        if item.is_none() && faraday_codec::ur::is_ur(&text) {
            let message = match ur_feed(&mut scan.decoder, &text) {
                Ok(UrFed::Whole(m)) => m,
                Ok(UrFed::More { restarted }) => {
                    scan.note = ur_note(&scan.decoder, restarted);
                    return None;
                }
                Err(why) => {
                    scan.note = Some(why.to_string());
                    return None;
                }
            };
            match message {
                faraday_codec::ur::Message::Psbt(p) => item = Some((None, "psbt", p)),
                faraday_codec::ur::Message::Bytes(b) => {
                    item = Some(arrival(faraday_qr::classify_text(
                        &String::from_utf8_lossy(&b),
                    )));
                }
                faraday_codec::ur::Message::Other { ur_type, cbor } => {
                    match faraday_qr::registry::read(&ur_type, &cbor) {
                        Ok(Some(faraday_qr::registry::Read::Psbt(p))) => {
                            item = Some((None, "psbt", p));
                        }
                        Ok(Some(faraday_qr::registry::Read::Keys(t)))
                        | Ok(Some(faraday_qr::registry::Read::Descriptor(t))) => {
                            item = Some((None, "txt", t.into_bytes()));
                        }
                        Ok(None) => {
                            scan.note = Some(fill(EN.read_code_ur_is_not, &[&ur_type]));
                            return None;
                        }
                        Err(why) => {
                            scan.note = Some(why);
                            return None;
                        }
                    }
                }
            }
        }
        let (named, ext, data) = match item {
            Some(i) => i,
            None if wallet::read_psbt(&c, &bytes).is_some() => (None, "psbt", bytes.to_vec()),
            None => {
                // An address is checked against the loaded wallets, not
                // filed.
                if let Some(said) = self.address_note(&c, &text) {
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

    /// The camera gives no picture, and the shell said so: the sheet stays
    /// with the reason in its square and its other ways in live
    /// (`docs/QR.md` §8 item 6).
    fn camera_failed(&mut self, why: NoPicture) {
        if let Some(s) = self.scan.as_mut() {
            s.no_picture = Some(why);
            s.waiting_since = None;
        }
        self.release_held_stick();
    }

    /// The scanner's clock, on each tick: the camera that sends no
    /// picture, and "Nothing to paste" running out. Whether the sheet
    /// changed.
    fn scan_tick(&mut self, now_ms: u64) -> bool {
        let Some(s) = self.scan.as_mut() else {
            return false;
        };
        let mut changed = false;
        if s.nothing_to_paste.is_some_and(|until| now_ms >= until) {
            s.nothing_to_paste = None;
            changed = true;
        }
        if self.sheet == Some(Sheet::Scan) && s.frame.is_none() && s.no_picture.is_none() {
            match s.waiting_since {
                None => s.waiting_since = Some(now_ms),
                Some(since)
                    if now_ms.saturating_sub(since) >= faraday_draw::tokens::NO_PICTURE_MS =>
                {
                    s.no_picture = Some(NoPicture::Silent);
                    changed = true;
                }
                Some(_) => {}
            }
        }
        changed
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
                EN.address_note_that_address_is_not,
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
                    EN.address_note_that_address_is,
                    &[&w.name.to_string(), chain, &index.to_string()],
                ));
            }
        }
        Some(if self.session.wallets.is_empty() {
            EN.address_note_that_is.to_string()
        } else {
            EN.address_note_that_address.to_string()
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

    /// Costs what `text`, `wallet_file_text`, `create_key_text` and the
    /// rest it calls cost. The QR view of a public file a flow makes: its
    /// code, and the name and label of its picture.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub fn code_view(&self, c: &Compute, code: Code) -> Result<QrView, String> {
        let wallet = |i: usize| {
            self.session
                .wallets
                .get(i)
                .ok_or_else(|| EN.code_view_that_wallet.to_string())
        };
        match code {
            Code::Descriptor(i) => {
                let w = wallet(i)?;
                let stem = file_stem(&w.name);
                let mut lines = self.wallet_label(w);
                let (title, text, name) = match w.policy().record() {
                    Some(r) => (
                        fill(EN.code_view_threshold, &[&w.name.to_string()]),
                        r.to_text(),
                        format!("{stem}-record.png"),
                    ),
                    None => {
                        let text = w.policy().to_descriptor_checksummed();
                        if let Some((_, sum)) = text.rsplit_once('#') {
                            lines.push(fill(EN.code_view_descriptor, &[sum]));
                        }
                        (
                            fill(EN.code_view_wallet_descriptor, &[&w.name.to_string()]),
                            text,
                            format!("{stem}-descriptor.png"),
                        )
                    }
                };
                lines.push(EN.code_view_public_watch.to_string());
                let mut view = QrView::text(c, &title, &text)?.public(&name, lines);
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
                    .wallet_file_text(c, i, what)
                    .ok_or(EN.code_view_this_wallet_has)?;
                let (title, name) = if what == 2 {
                    (
                        fill(EN.code_view_multisig, &[&w.name.to_string()]),
                        format!("{stem}-multisig-config.png"),
                    )
                } else {
                    (
                        fill(EN.code_view_bsms_descriptor, &[&w.name.to_string()]),
                        format!("{stem}-bsms.png"),
                    )
                };
                let mut lines = self.wallet_label(w);
                lines.push(EN.code_view_public_watch.to_string());
                Ok(QrView::text(c, &title, &text)?.public(&name, lines))
            }
            Code::Key(k) => {
                let (fp, text) = self.create_key_text(c, k)?;
                let kind = self.create.as_ref().map_or("", |c| c.kind.name());
                let account = self.create_key_account(c, k);
                let lines = vec![
                    key_label(&fp, kind, &text),
                    EN.code_view_public_spends.to_string(),
                ];
                Ok(QrView::of(
                    c,
                    &fill(EN.code_view_xpub_2, &[&fp.to_string(), kind]),
                    QrSource::Key(text, account),
                    QrFormat::Ur,
                    QR_PARTS[1],
                )?
                .public(&format!("xpub-{fp}.png"), lines))
            }
            Code::KeyBsms(k) => {
                let (fp, key) = self.create_key_text(c, k)?;
                let kind = self.create.as_ref().map_or("", |c| c.kind.name());
                let (_, text) = self
                    .create_key_bsms(c, k)
                    .ok_or(EN.code_view_bip_129_covers)?;
                let lines = vec![
                    key_label(&fp, kind, &key),
                    EN.code_view_bip_129_key.to_string(),
                    EN.code_view_public_spends.to_string(),
                ];
                Ok(
                    QrView::text(c, &fill(EN.code_view_key_bsms, &[&fp.to_string()]), &text)?
                        .public(&format!("xpub-{fp}-bsms.png"), lines),
                )
            }
            Code::WalletKey(i, slot) => {
                let k = self.wallet_key(c, i, slot)?;
                let account = k
                    .standard
                    .and_then(|kind| kind.account_ur(c, &self.session.keys, k.key));
                let lines = vec![
                    key_label(&k.fp, &k.kind, &k.text),
                    EN.code_view_public_spends.to_string(),
                ];
                Ok(QrView::of(
                    c,
                    &fill(
                        EN.code_view_xpub_2,
                        &[&k.fp.to_string(), &k.kind.to_string()],
                    ),
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
                    .ok_or(EN.code_view_that_wallet)?;
                if w.policy().record().is_some() || w.policy().silent().is_some() {
                    return Err(EN.code_view_this_wallet_s.into());
                }
                let pk = w
                    .policy()
                    .keys()
                    .get(usize::from(slot))
                    .ok_or(EN.code_view_this_wallet)?
                    .clone();
                let fp = pk
                    .fingerprint()
                    .map_or_else(|| EN.code_view_no_origin.to_string(), fp_text);
                let text = pk.key_text();
                let lines = vec![
                    fill(
                        EN.code_view_key,
                        &[&fp.to_string(), &Session::shape(w).to_string()],
                    ),
                    EN.code_view_public_spends.to_string(),
                ];
                Ok(QrView::of(
                    c,
                    &fill(EN.code_view_xpub, &[&fp.to_string()]),
                    QrSource::Key(text, None),
                    QrFormat::Ur,
                    QR_PARTS[1],
                )?
                .public(&format!("xpub-{fp}.png"), lines))
            }
            Code::WalletKeyBsms(i, slot) => {
                let k = self.wallet_key(c, i, slot)?;
                let (_, text) = self.wallet_key_record(c, i, slot)?;
                let lines = vec![
                    key_label(&k.fp, &k.kind, &k.text),
                    EN.code_view_bip_129_key.to_string(),
                    EN.code_view_public_spends.to_string(),
                ];
                Ok(
                    QrView::text(c, &fill(EN.code_view_key_bsms, &[&k.fp.to_string()]), &text)?
                        .public(&format!("xpub-{}-bsms.png", k.fp), lines),
                )
            }
            Code::Silent(_) | Code::SilentRecord => self.silent_code(c, code),
            Code::Message => {
                let m = self.message.as_ref().ok_or(EN.code_view_no_message)?;
                let sig = m.signed.as_ref().ok_or(EN.code_view_the_message)?;
                let text =
                    faraday_keys::message::signed_text(&sig.address, &sig.signature, &m.text);
                let name = format!(
                    "message-{}.png",
                    &sig.address[sig.address.len().saturating_sub(6)..]
                );
                let lines = vec![
                    fill(EN.code_view_signed_by, &[&sig.address.to_string()]),
                    EN.code_view_public_proves.to_string(),
                ];
                Ok(QrView::text(c, EN.code_view_signed_message, &text)?.public(&name, lines))
            }
            Code::GpgKey | Code::GpgSignature(_) => self.gpg_code(c, code),
            Code::ApkCertificate => self.apk_code(c),
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

    /// Costs an address derivation (through `address_at`). A wallet's
    /// public text file `what` (as [`Faraday::public_out`] numbers them): 2
    /// the multisig config, 6 the BIP 129 record.
    fn wallet_file_text(&self, c: &Compute, wallet: usize, what: u8) -> Option<String> {
        let w = self.session.wallets.get(wallet)?;
        match what {
            2 => wallet_backup::multisig_config(w, None),
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
        match self.entry.back {
            Some(s @ (Screen::Family | Screen::Spend)) => s,
            Some(Screen::Restore) if self.restore.as_ref().is_some_and(|r| r.seeds.is_some()) => {
                Screen::Restore
            }
            _ => Screen::Wallets,
        }
    }

    /// The words typed on Add a key's words form, the word being typed
    /// on a list's own keyboard taken first, as a key's words: whole and
    /// with a checksum that holds, or what is wrong with them.
    pub(crate) fn entry_mnemonic(&mut self) -> Result<faraday_bip::bip39::Mnemonic, String> {
        let lang = self.entry.language();
        if let Some(w) = self.entry.keys.as_mut() {
            forms::take_typed(w, lang);
        }
        match self.entry.keys.as_ref() {
            Some(w) => {
                let mut idx: Vec<u16> = w.committed_indices().collect();
                let m = faraday_keys::words::from_indices(lang, &idx).map_err(|e| e.to_string());
                zeroize::Zeroize::zeroize(&mut idx);
                m
            }
            None => forms::typed_mnemonic(lang, &self.entry.typed),
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

    /// Whether Add a key's form takes the keys typed: on Add a key, or on
    /// New key's Words card for a key typed (`docs/history/NEW-WALLET.md` §15.2).
    pub(crate) fn entry_typing_here(&self) -> bool {
        self.screen == Screen::Entry
            || (self.screen == Screen::KeyGen
                && self
                    .keygen
                    .as_ref()
                    .is_some_and(|k| k.open == Some(keygen::kstep::TYPED) && !k.locked))
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
        let wanted = self.entry.wanted.map(faraday_bip::keys::Fingerprint);
        let label = self.entry.label.clone();
        let typed = self.entry_mnemonic();
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
        // Add a key for a wallet takes only a key the wallet is missing
        // (`docs/history/SCAN-TO-SIGN.md` §2.1).
        let mut label = label;
        let mut for_wallet = None;
        if let Some(w) = self.entry.wallet {
            let fp =
                faraday_keys::identity::fingerprint_of_mnemonic(&c, &m, &self.entry.passphrase)
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
                        self.entry.error = Some(
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
        if back == Some(Screen::Restore) {
            let fp =
                faraday_keys::identity::fingerprint_of_mnemonic(&c, &m, &self.entry.passphrase)
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
        let pass = secret_text::SecretText::of(self.entry.passphrase.as_str());
        let said = |app: &mut Self, fp: faraday_bip::keys::Fingerprint| match for_wallet {
            Some((w, n)) => app.wallet_key_said(w, n),
            None => app.toast(&fill(EN.entry_add_key_added, &[&fp_text(fp).to_string()])),
        };
        match self.typed_load(&m, &pass, &label, wanted, back, said) {
            Ok(_) => {
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
            // A seed already loaded comes in on the Spend tab's What do
            // you have? as it is (`docs/history/SCAN-TO-SIGN.md` §4).
            Err(wallet::Refusal::Duplicate(_)) if back == Some(Screen::Family) => {
                if let Ok(fp) =
                    faraday_keys::identity::fingerprint_of_mnemonic(&c, &m, &self.entry.passphrase)
                {
                    self.seeds_took(fp, back);
                    self.entry = EntryState::default();
                    self.refresh_spend();
                    self.screen = Screen::Family;
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
        let c = self.compute();
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
        let name = wallet::json_label(&text).unwrap_or(name);
        match self
            .session
            .add_wallet(&c, &name, &text, &source, wallet::Origin::Named)
        {
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
        self.sticks.get(self.visit.stick).is_some_and(|s| {
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
            (Some(FileKind::Other), _) => EN.visit_inbox_a_png_with.to_string(),
            (Some(k), _) => fill(
                EN.visit_inbox_a_png_of,
                &[&screens::kind_name(k).to_lowercase().to_string()],
            ),
            (None, FileKind::Other) => EN.visit_inbox_a_file.to_string(),
            (None, k) => screens::kind_name(k).to_string(),
        };
        let gives = match holds {
            FileKind::Words => EN.visit_inbox_whoever_has_it_holds,
            FileKind::SeedPart => EN.visit_inbox_whoever_has_enough,
            FileKind::Entries => EN.visit_inbox_whoever_has_it,
            FileKind::Carry => EN.visit_inbox_whoever_has,
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
        if let Some(stick) = self.sticks.get(self.visit.stick) {
            let id = stick.id.clone();
            self.visit.log.clear();
            for name in self.visit.inn.clone() {
                // A picture's QR codes are read, not its bytes.
                let stick = id.clone();
                self.visit.reading.insert(name.clone());
                self.storage_out
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
                s.error = Some(fill(
                    EN.sign_here_waits_for,
                    &[&waiting.join(", ").to_string()],
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
                    s.error = Some(EN.sign_here_no_randomness.to_string());
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
                s.error = Some(EN.sign_here_this_transaction.to_string());
            }
            return;
        }
        let mut remember: Option<faraday_psbt::Psbt> = None;
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
            s.error = Some(fill(
                EN.sign_here_choose_other,
                &[
                    &want.to_string(),
                    (if want == 1 {
                        EN.sign_here_share
                    } else {
                        EN.sign_here_shares
                    }),
                ],
            ));
            s.open = Some(wallet::step::SIGNERS);
            s.follow = true;
            return;
        }
        let mut psbt = s.spend.psbt.clone();
        let mut nonce_only = false;
        // A threshold pass signs as the shares held here; it names them
        // by their own fingerprints.
        let ours: Option<Vec<faraday_bip::keys::Fingerprint>> = s
            .inspection
            .inputs
            .first()
            .and_then(|i| i.threshold.as_ref())
            .and_then(|t| {
                let record = self.session.wallets.get(s.wallet?)?.policy().record()?;
                Some(
                    t.ours
                        .iter()
                        .filter_map(|id| record.share_fingerprint(*id as usize))
                        .collect(),
                )
            });
        // The spend's inspection chooses the keys when it is of this
        // PSBT and session as they are; else signing inspects.
        let inputs = self.session.inspect_inputs();
        let known = s.inspected_for(&inputs).then_some(&s.inspection);
        let signed = self.session.sign_with(
            &c,
            &mut psbt,
            seed,
            &mut s.musig,
            &s.others,
            s.carry.as_ref(),
            known,
        );
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
                s.carry_out = Some(faraday_keys::carry::bytes(&section, &psbt.to_bytes()));
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
            self.toast(EN.sign_here_this_device);
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
        let c = self.compute();
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
            match seed {
                Some(seed) => {
                    if self.session.sign(&c, &mut psbt, seed, &mut s.musig).is_ok() {
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
        let can_sign_here =
            match threshold {
                Some(t) => t.ours.iter().any(|id| !t.signed.contains(id)),
                None => s.inspection.participating_keys.iter().any(|fp| {
                    self.session.keys.key_label(*fp).is_some() && !s.signers.contains(&fp.0)
                }),
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
        let Some(at) = self.typed_at else {
            return false;
        };
        if now_ms < at + motion::TYPED_PAUSE_MS {
            return false;
        }
        self.typed_at = None;
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
        self.typed_at.is_some()
    }

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn key(&mut self, key: KeyIn) {
        if self.selftest_failed() || self.pass_typed(key) {
            return;
        }
        // Ctrl+V is the scanner's Paste, and nothing anywhere else.
        if key == KeyIn::Paste {
            self.scan_paste();
            return;
        }
        if self.wordlist_key(key) {
            return;
        }
        if self.board_key(key) {
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
        if self.file_key(key) {
            return;
        }
        if self.seed_read_key(key) {
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
        if self.bip85_key(key) {
            return;
        }
        if self.keygen_key(key) {
            return;
        }
        if self.create_pass_key(key) {
            return;
        }
        if self.set_time_key(key) {
            return;
        }
        if self.screen == Screen::Entry && self.entry.on_passphrase {
            match key {
                // BIP-39 passphrases here are printable ASCII (BIP-39's seed derivation).
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
        let here = self.entry_typing_key();
        if here && self.entry.form != forms::Form::Words {
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
                KeyIn::Enter if self.entry.typed.trim().is_empty() => {
                    if self.screen == Screen::KeyGen {
                        self.keygen_act(Action::KNext);
                    } else {
                        self.form_recover();
                    }
                }
                KeyIn::Enter => self.form_add_part(),
                KeyIn::Escape => {
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
            let lang = self.entry.language();
            let typed = forms::word_key(
                forms::WordsMut {
                    typed: &mut self.entry.typed,
                    keys: &mut self.entry.keys,
                    error: &mut self.entry.error,
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
        // The words of a copy, typed back for Check my copy.
        if self.screen == Screen::Backup && self.backup.as_ref().is_some_and(|b| b.check.is_some())
        {
            self.copy_check_key(key);
            return;
        }
        // A place's name is typed on Back up's plan, and on a wallet
        // flow's plan card before its lock (`docs/BACKUP-FLOW.md` §15.7).
        let naming_in_flow = self.screen == Screen::BackupFlow
            && self.backup.as_ref().is_some_and(|b| b.naming.is_some());
        if (self.screen == Screen::Backup || naming_in_flow)
            && let Some(b) = self.backup.as_mut()
        {
            match key {
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
            KeyIn::Down => self.glide(motion::WHEEL_NOTCH),
            KeyIn::Up => self.glide(-motion::WHEEL_NOTCH),
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
            ui::Slot::FilesOut => Some(&mut self.files_out_offset),
            ui::Slot::BoardList => Some(&mut self.board_list_offset),
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
            ui::Slot::FilesOut
                if self.screen == Screen::Files
                    && !self.compact
                    && self.region_key().1.is_none() =>
            {
                ui::Slot::FilesOut
            }
            ui::Slot::BoardList
                if self.board_shown() && !self.compact && self.region_key().1.is_none() =>
            {
                ui::Slot::BoardList
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

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
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
                self.vault_seedqr_press(found.map(|f| f.0));
                // A press on a chip of the backup board may be its drag:
                // the page does not scroll under it (§19.2).
                if self.board_down(found.map(|f| f.0), x, y) {
                    self.drag = None;
                }
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
                // A board chip past the slop follows the finger.
                if self.board_move(x, y) {
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
                // A board chip dragged is dropped where it is lifted.
                if self.board_up(x, y) {
                    self.drag = None;
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
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
                self.vaults.seedqr = None;
                // A hold acts on the tick that completes it, never on release.
                if let (Some((a, _)), Some((b, _))) = (pressed, found)
                    && a == b
                    && !vault::is_hold(a)
                {
                    // On a field: the caret where it was pressed, a
                    // second press selecting all of it, a drag what it
                    // went over (`docs/history/NEW-WALLET.md` §13.2).
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

    /// Works out, before a frame is drawn, what is kept read on the state
    /// and was dropped by a change since the last frame, so that drawing
    /// only looks it up (`docs/SPEED.md` §3.3, `docs/COMPUTE.md` §4):
    /// each open vault's records read again when they changed. Called
    /// before every [`Self::render`].
    fn before_frame(&mut self) {
        let c = self.compute();
        for o in &self.vaults.open {
            self.vaults.warm(&c, o.id);
        }
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
        let mut hits = std::mem::take(&mut self.hits);
        hits.clear();
        // Offsets in whole pixels, whoever set them last, so a scrolled
        // page moves as one.
        let f = self.f.max(0.1);
        let whole = self.whole_offset();
        if let Some(offset) = self.scroll_slot() {
            *offset = whole(*offset);
        }
        let stretch = (self.motion.stretch() * f).round() as i32;
        // The overlay scrollbar shows while the offset moves and fades
        // once it has rested a moment.
        let key = self.region_key();
        let slot = self.slot();
        let offset = self.scroll_slot().map_or(0.0, |o| *o);
        let page_offset = self.screen_slot().map_or(0.0, |o| *o);
        self.sheet_offset = self.page_slot().map_or(0.0, |o| *o);
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
        // A field newly focused brings the keyboard up; on a small panel
        // the page scrolls so the field stays in view above it. Worked
        // out before drawing, which reads the keyboard as it now is.
        if self.osk_track() && self.compact {
            let kb = self.osk_height();
            if kb > 0.0 {
                self.osk_reveal(self.f, self.h - kb);
            }
        }
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
                ui.dimmed = self.demo_dimmed();
                ui.stretch = stretch;
                ui.bar = (bar > 0).then_some((bar, offset));
                ui.stretch_in_sheet = matches!(
                    self.sheet,
                    Some(Sheet::Learn | Sheet::WordList | Sheet::Import | Sheet::UnlockLoad)
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
                let view = drawing::View::new(self);
                screens::draw(&view, &mut ui);
                self.apply_after(view.into_after());
                caret = ui.caret_drawn.then_some(ui.caret_on);
                self.osk_caret(ui.caret_foot);
                self.fields = std::mem::take(&mut ui.fields);
                self.back_link = ui.back_link.map(|a| (self.screen, a));
                self.back_hit = ui.back_hit;
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
        self.later_drawn = true;
        self.hits = hits;
        self.fade_end(&mut canvas);
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
        let whole = self.whole_offset();
        let offset = self.scroll_slot().map_or(0.0, |o| whole(*o));
        self.bar_seen = ((self.region_key(), self.slot()), offset, 0);
        if self.toast.is_some() {
            self.toast_at = Some(self.now_ms.saturating_sub(motion::TOAST_MS));
        }
        // The next frame is the screen at rest, not part of the way from
        // the one before.
        self.fade.end();
        self.last_drawn = None;
        self.dirty = true;
    }

    /// Acts as if `action` had been pressed: for drivers that script the
    /// screens, such as the snapshot tool.
    pub fn press(&mut self, action: Action) {
        // An action may change the words typed: the status line's key is
        // worked out again, once typing pauses when the action is a
        // character from the on-screen keyboard.
        self.entry.forget_probe();
        if matches!(
            action,
            Action::EntryKey(_) | Action::EntryKeyBack | Action::EntryCandidate(_)
        ) && self.entry_typing_here()
        {
            self.typed_at = Some(self.now_ms);
        }
        self.note_secrets();
        self.note_here();
        self.press_in(action);
        // A script does not wait for frames: what a person's press leaves
        // for the tick after its frame is done now.
        self.later_now();
        self.backup_settle();
        self.after_input();
        self.dirty = true;
        self.commands.push_back(Command::Draw);
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
            self.dirty = true;
            self.commands.push_back(Command::Draw);
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
        self.toast_at = Some(self.now_ms.saturating_sub(motion::TOAST_MS));
    }

    /// Whether `key` is left for the tick after a frame: Enter where it
    /// reads an aezeed, which scrypt stretches for seconds on a Pi.
    fn key_slow(&self, key: KeyIn) -> bool {
        key == KeyIn::Enter && self.screen == Screen::Lightning && self.lightning_reads_aezeed()
    }

    /// Does the press waiting, once a frame has been drawn since it was
    /// pressed. Public files a slider asked for wait until it is let go.
    /// Returns whether it did.
    fn later_tick(&mut self) -> bool {
        let held = matches!(self.pressed, Some((Action::Slide(..), _)));
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
        let toast = self.toast.as_ref().map(|(t, _)| t.clone());
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
            && self.toast.as_ref().map(|(t, _)| t.clone()) == toast
            && [EN.working_files, EN.working_node_key]
                .iter()
                .any(|w| toast.as_deref() == Some(*w))
        {
            self.toast = None;
        }
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    /// What every press does once it has acted, however it came (a tap,
    /// a key, the mouse's back button, a scripted press): a screen left
    /// drops what it held, a detour not taken ends, a spent round goes
    /// from its vault, and a sheet waiting comes up. Lookups, unless
    /// something is dropped.
    fn after_input(&mut self) {
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
        // A vault's passphrase, typed or half typed, goes once nothing that
        // takes it is up, however it was left; an unlock under way keeps it
        // until it is done.
        let typing = matches!(
            self.screen,
            Screen::Unlock | Screen::BackupFlow | Screen::Family
        ) || self.sheet == Some(Sheet::Import);
        if !typing && self.vaults.working != Some(vault::Work::Unlock) {
            self.vaults.passphrase.clear();
            self.vaults.typed_shown = false;
            self.vaults.unlock_error = None;
        }
        // Leaving the new-key screen drops what it held.
        if self.screen != Screen::KeyGen && self.keygen.is_some() {
            self.keygen_drop();
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
        self.device_sheet();
    }

    /// Whether the last frame drawn offers `action` to a press: for
    /// tests that a control is on screen.
    pub fn offers(&self, action: Action) -> bool {
        self.hits.iter().any(|(_, a)| *a == action)
    }

    /// The time of the last tick, ms: what a test's next tick follows.
    pub fn now_ms(&self) -> u64 {
        self.now_ms
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
    pub fn loose_keys(&self) -> (Vec<faraday_bip::keys::Fingerprint>, Option<[u8; 4]>) {
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
    v.extend([cstep::WHERE, cstep::KEYS, cstep::CHECK, cstep::BACKUP]);
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

/// What a press leaves for the tick after the frame that shows it
/// (`docs/SPEED.md` §9).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Later {
    /// A press whose work is [`slow`].
    Press(Action),
    /// A key whose work is slow (`Faraday::key_slow`).
    Key(KeyIn),
    /// The backup's public files of kind `what` for wallet `w`, made again
    /// once a slider that changes them is let go.
    PublicOut(usize, u8),
}

/// Whether a press stretches a key (PBKDF2, scrypt, Argon2) or makes more
/// than one picture or PDF, so that the frame that shows it is drawn
/// first and its work is done on the tick after (`docs/SPEED.md` §9). A
/// press that does such work and is not here fails the guard in
/// `Faraday::press_in` in every test.
fn slow(a: Action) -> bool {
    use crate::backup_flow::steps::FlowAction as F;
    use crate::glance::sheet::ChartAction as C;
    use crate::vault::VaultAction as V;
    matches!(
        a,
        Action::BChecklist
            | Action::BCodex32(..)
            | Action::BNewVault(_)
            | Action::CNext(_)
            | Action::CPassLock
            | Action::CSlotVault(..)
            | Action::EntryAdd
            | Action::EntryRecover
            | Action::Flow(F::CopyKey | F::LockWrite | F::WalletLock)
            | Action::Import(boot_import::ImportAction::Go)
            | Action::InboxLoad
            | Action::KLock
            | Action::KNext
            | Action::LKey(_)
            | Action::LResolve
            | Action::LoadKey(_)
            | Action::Lock
            | Action::SamplesLoad
            | Action::ScanSample
            | Action::PLoad
            | Action::PotentialMake
            | Action::SecretUnprotected
            | Action::SeedRead(seeds::read::Choice::Plain | seeds::read::Choice::WithPass)
            | Action::Seeds(seeds::SeedsAction::Make | seeds::SeedsAction::Sample(_))
            | Action::SelfTestRun
            | Action::Vanity(vanity::VanityAction::Use)
            | Action::Chart(
                C::Checked(..) | C::Confirm(_) | C::Do(..) | C::Edit(..) | C::PassTry(_)
            )
            | Action::Vault(
                V::AddFile(_)
                    | V::AddKind(_)
                    | V::Category(_)
                    | V::LoadAll(_)
                    | V::LoadChosen(_)
                    | V::LoadKeyOf(..)
                    | V::LoadTicked
                    | V::LoadWithKeys(_)
                    | V::Open(_)
                    | V::OpenWalletOf(..)
                    | V::PromptGo
                    | V::SaveKey(_)
                    | V::SaveKeyWithPassphrase(_)
                    | V::SaveWallet(_)
            )
    )
}

/// What the frame drawn before a slow press's work says, when the work
/// is long enough to be seen: pictures and PDFs made.
fn working_text(a: Action) -> Option<&'static str> {
    use crate::backup_flow::steps::FlowAction as F;
    matches!(
        a,
        Action::BChecklist | Action::SecretUnprotected | Action::Flow(F::WalletLock)
    )
    .then_some(EN.working_files)
}

fn field_action(a: Action) -> bool {
    use vault::VaultAction as V;
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
            | Action::SeedRead(seeds::read::Choice::Field(_))
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
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn event(&mut self, event: Event) {
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
            self.entry.forget_probe();
            // A character typed into the words or the passphrase: the key
            // they make is worked out once typing pauses.
            if matches!(event, Event::Key(_)) && self.entry_typing_here() {
                self.typed_at = Some(self.now_ms);
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
                } else if self.key_slow(k) {
                    // Drawn first, done on the tick after (`docs/SPEED.md`
                    // §9).
                    self.later_now();
                    self.later = Some(Later::Key(k));
                    self.later_drawn = false;
                    self.working(EN.working_node_key);
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                    return;
                } else {
                    self.later_now();
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
                // The shells send 48 pixels a notch; the notch's distance
                // is this layout's, in units, so it covers the same amount
                // of page on every screen.
                self.glide(f32::from(dy) / motion::SHELL_NOTCH_PX * motion::WHEEL_NOTCH);
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
                // A fade under way runs to its end whatever the gaps
                // say. Its clock starts now if the frame that showed the
                // change was the last one, a frame back, so this frame
                // is the first step of it rather than the old screen
                // again.
                if let Some((_, at @ None)) = self.fade.from.as_mut() {
                    *at = Some(now_ms.saturating_sub(motion::FADE_FIRST_STEP_MS));
                }
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
                // A fade under way asks for its next step; unless
                // something else asks for a frame too, the step blends
                // the two kept frames (`Faraday::fade_step`).
                if self.fade.from.is_some() {
                    self.fade.due = true;
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
                // Each runs whatever the others return.
                if self.later_tick()
                    | self.seed_tick()
                    | self.typed_tick(now_ms)
                    | self.explore_tick(now_ms)
                {
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                }
                // A live clock or a TOTP code on screen moves on by the
                // second, with nothing else moving.
                if self.shows_clock() {
                    let now = self.clock_now();
                    if now != self.clock_drawn {
                        self.clock_drawn = now;
                        self.dirty = true;
                        self.commands.push_back(Command::Draw);
                    }
                }
                self.vanity_tick(now_ms);
                if self.scan_tick(now_ms) {
                    self.dirty = true;
                    self.commands.push_back(Command::Draw);
                }
                self.zero_tick();
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
            Event::ClipboardUnavailable { .. } => {
                self.pasted(zeroize::Zeroizing::new(String::new()))
            }
            _ => return,
        }
        if settle {
            self.backup_settle();
            self.after_input();
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
            self.before_frame();
            self.render();
        } else if self.fade.due {
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
    /// The strings the current frame draws, for a test to check what a
    /// title or a label actually reads.
    pub fn drawn_texts(&mut self) -> Vec<String> {
        self.drawn_text_boxes()
            .into_iter()
            .map(|(t, _)| t)
            .collect()
    }

    /// The strings the current frame draws, each with the box of its ink
    /// in pixels (x, y, w, h): for a test of where a line is put, such as
    /// whether a row of labels fits on one line.
    pub fn drawn_text_boxes(&mut self) -> Vec<(String, (i32, i32, i32, i32))> {
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(true);
        }
        self.before_frame();
        self.render();
        let texts = self
            .canvas
            .as_ref()
            .map(|c| {
                c.ink()
                    .iter()
                    .filter_map(|i| match &i.kind {
                        faraday_draw::canvas::InkKind::Text { text, .. } => {
                            Some((text.clone(), (i.rect.x, i.rect.y, i.rect.w, i.rect.h)))
                        }
                        faraday_draw::canvas::InkKind::Icon(_) => None,
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
        let c = self.compute();
        self.last_display = Some(d);
        // The self-test runs once, at start, before any input: a display
        // comes before anything can be pressed.
        if self.selftest.is_none() {
            self.selftest = Some(faraday_selftest::run(&c));
        }
        self.frost = None;
        self.fade.end();
        self.last_drawn = None;
        // A fade's step costs a blend of the whole canvas: what it cost
        // at another size says nothing of this one.
        self.fade.gaps.clear();
        // Fill a small display; on a large one, stop at a quarter more
        // than its density asks for, so a 4K monitor shows the screens
        // at a sensible size rather than magnified. The person's own
        // choice multiplies whatever that gives.
        let fit = (f32::from(d.width) / 1280.0).min(f32::from(d.height) / 800.0);
        let density = f32::from(d.dpi.max(1)) / 160.0;
        // A panel under 600 dp wide (the Pi's 2.8" is 268) gets the
        // one-column layout at its own density, not the desktop shrunk.
        // So does a window the wide layout would draw below three quarters
        // of its density, counted up to `WIDE_FLOOR_DENSITY` so that above
        // it the window's pixels decide (F78): 960 × 640 at 160 dpi is wide.
        let floor = density.min(faraday_draw::tokens::WIDE_FLOOR_DENSITY) * 0.75;
        self.compact = f32::from(d.width) / density < 600.0 || fit < floor;
        // Held inside the Starter, the screens draw at the desktop's own
        // density, and take the one-column layout in any window narrower
        // than the wide layout's 1280 dp, rather than shrinking it.
        if self.held && f32::from(d.width) / density < 1280.0 {
            self.compact = true;
        }
        let base = if self.compact || self.held {
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
        let mut canvas = Canvas::new(&scaled);
        // The canvas notes its pixel work, a blur or a whole copy, with
        // this crate's counter (`docs/COMPUTE.md` §2.2).
        canvas.set_counter(work::core_count);
        self.canvas = Some(canvas);
        self.fresh_ask();
    }
}

/// Whether a pass over this transaction may open a MuSig2 or FROST
/// round, which draws secret nonces: any such input is one, since what
/// the pass does with it is decided only as it signs.
fn spend_opens_round(inspection: &faraday_psbt::Inspection) -> bool {
    inspection
        .inputs
        .iter()
        .any(|i| i.musig.is_some() || i.threshold.is_some())
}
