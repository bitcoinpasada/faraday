//! Vaults in the app (`docs/VAULT.md`, `PLAN.md` §5.3 and §6.1): the
//! vaults the Inbox and Outbox hold, unlocking one slot of one, its
//! contents by kind, creating a vault, and sealing what changed when the
//! session locks.
//!
//! A vault file arrives in the Inbox on a stick visit, or is made here and
//! waits in the Outbox. Unlocking is never offered while a stick is
//! attached. An open slot is a secret: it counts for the stick rule, and
//! locking seals it into the Outbox when it changed. The slow part of
//! unlocking and creating, Argon2id, runs on the tick after a frame that
//! says it is working has been drawn.

use crate::strings::EN;
use faraday_vault::records::{self, field, kind};
use faraday_vault::{self as fv, Contents, Cost, Header, Record};
use opensigner_core::strings::fill;
use osk_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
use std::collections::BTreeSet;
use zeroize::Zeroizing;

use crate::{Action, Faraday, FileKind, Screen, flow};

/// The presets of `docs/VAULT.md` §3.1: name, memory in MiB, passes, in
/// order of memory.
pub const PRESETS: [(&str, u32, u32); 5] = [
    (EN.vaults_light, 64, 3),
    (EN.vaults_standard, 256, 3),
    (EN.vaults_medium, 512, 3),
    (EN.vaults_strong, 1024, 4),
    (EN.vaults_maximum, 2048, 2),
];

/// The place after the presets that stands for a custom cost.
pub const CUSTOM: usize = PRESETS.len();

/// Create a vault's two sizes (`docs/SIMPLIFY.md` §3.1): the label, the
/// preset it takes, and the machines it is for besides this one, in
/// `MACHINES` order. The second is the default.
pub const SIZES: [(&str, usize, [bool; 5]); 2] = [
    (EN.vaults_pcs_only, 2, [true, false, false, false, false]),
    (
        EN.vaults_pcs_and_a_raspberry,
        0,
        [true, false, false, true, false],
    ),
];

/// The size a new vault starts on: PCs and a Raspberry Pi.
pub const DEFAULT_SIZE: usize = 1;

/// The machines a vault may be opened on, besides this one: label, short
/// name, memory shown, memory in MiB.
pub const MACHINES: [(&str, &str, &str, u32); 5] = [
    (
        EN.vaults_other_pcs,
        EN.vaults_other_pcs_inline,
        "4 GB",
        4096,
    ),
    (EN.vaults_raspberry_pi_5, EN.vaults_pi_5, "4 GB", 4096),
    (EN.vaults_raspberry_pi_4, EN.vaults_pi_4, "2 GB", 2048),
    (EN.vaults_raspberry_pi_3, EN.vaults_pi_3, "1 GB", 1024),
    (EN.vaults_pi_zero_2_w, EN.vaults_pi_zero_2_w, "512 MB", 512),
];

/// The most unlock memory This computer is suggested for, MiB: more
/// buys a vault little.
pub const MAX_UNLOCK_MIB: u32 = 4096;
/// What the app itself takes beside a vault's unlock memory and file,
/// MiB: a slot's plaintext and the app.
pub const APP_MIB: u32 = 100;

/// The memory a machine needs free to open a vault of this unlock memory
/// and slot size, MiB: the Argon2id memory, twice the file (the file and
/// a sealed copy beside it while it is written) and [`APP_MIB`]
/// (`docs/VAULT.md` §3.1).
pub fn vault_need_mib(memory_mib: u32, slot_len: u32) -> u32 {
    const MIB: u64 = 1024 * 1024;
    let twice = (2 * fv::file_len(slot_len) as u64 + MIB / 2) / MIB;
    memory_mib + twice as u32 + APP_MIB
}

/// A custom cost's memory choices, MiB.
pub const CUSTOM_MEMORY: [u32; 8] = [64, 128, 256, 512, 1024, 2048, 3072, 4096];
/// A custom cost's pass choices.
pub const CUSTOM_PASSES: [u32; 7] = [2, 3, 4, 5, 6, 8, 10];

/// The kinds a vault's contents are listed by, with the record types each
/// shows.
pub const CATEGORIES: [(&str, &[u8]); 7] = [
    (EN.vaults_bitcoin_keys, &[kind::KEY, kind::PASSPHRASE]),
    (EN.vaults_wallets, &[kind::WALLET, kind::WALLET_SHARE]),
    (EN.vaults_entries, &[kind::ENTRY]),
    (EN.vaults_notes, &[kind::NOTE, kind::SHEET]),
    (EN.vaults_gpg_keys, &[kind::GPG]),
    (EN.vaults_secure_boot, &[kind::SECURE_BOOT]),
    (EN.apk_signing_keys, &[kind::APK_KEY]),
];

/// The place of the APK signing keys in `CATEGORIES`.
pub const APK_CATEGORY: usize = 6;

/// Bits of a passphrase from dice counted strong: six words of the EFF
/// long list (`docs/VAULT.md` §3.1).
pub const STRONG_BITS: u32 = 77;

/// The words this list needs for a strong passphrase, said with where the
/// lists come from: text, since Faraday is offline.
pub fn dice_aim(list: osk_bip::diceware::List) -> String {
    let n = (1..=20)
        .find(|&n| list.bits(n) >= STRONG_BITS)
        .unwrap_or(20);
    fill(
        EN.vaults_aim_for_words,
        &[&n.to_string(), &STRONG_BITS.to_string()],
    )
}

/// The bits an unlock cost adds to every passphrase (`docs/VAULT.md`
/// §3.1): one guess is memory × passes of Argon2id's 1 KiB block steps,
/// and the cost adds the base-2 logarithm of that count. Doubling the
/// memory or the passes adds one bit. An estimate, shown with ≈: it
/// counts work, and leaves out how much harder memory is than time for
/// an attacker's hardware.
pub fn cost_bits(memory_mib: u32, passes: u32) -> f32 {
    (f64::from(memory_mib) * 1024.0 * f64::from(passes)).log2() as f32
}

/// How long "Hold to delete" is held before it deletes, in ms.
pub const HOLD_MS: u64 = 1200;

/// The steps of Create a vault.
pub mod vstep {
    /// Where it will be opened.
    pub const WHERE: u8 = 0;
    /// The Argon2id cost.
    pub const COST: u8 = 1;
    /// The slot size.
    pub const SIZE: u8 = 2;
    /// The passphrases.
    pub const PHRASES: u8 = 3;
    /// The size: one of two presets, standing for the first three steps
    /// until Customise opens them.
    pub const PRESET: u8 = 4;
    /// How many steps there are.
    pub const COUNT: usize = 5;
}

/// Everything a vault screen can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultAction {
    /// Open a vault from the list: its contents when open, else Unlock.
    Open(usize),
    /// Start Create a vault.
    Create,
    /// Start Create a vault from another flow, which it returns to.
    CreateFrom(Screen),
    /// Unlock vault n from another flow, which it returns to.
    OpenFrom(usize, Screen),
    /// The vault list, to unlock one of several for another flow, which
    /// the unlock returns to.
    ListFrom(Screen),
    /// Back from the vault list to the flow that made a vault.
    Back,
    /// Load the seed with this master fingerprint from whichever open
    /// vault holds it.
    LoadKeyOf([u8; 4]),
    /// Load the chosen wallets of open vault v with their seeds in it.
    LoadWithKeys(usize),
    /// From the vault's chart: load wallet record r of open vault v, with
    /// its seeds there where keys may load, and open its card.
    OpenWalletOf(usize, usize),
    /// From the vault's chart: load wallet record r of open vault v as
    /// [`VaultAction::OpenWalletOf`] does, and open its backup's
    /// checklist.
    ChecklistOf(usize, usize),
    /// Show or hide open vault v's seeds and wallets one by one.
    EachShown(usize),
    /// Choose or drop record n of open vault v for the next load.
    Choose(usize, usize),
    /// Load every key and wallet of open vault v.
    LoadAll(usize),
    /// Load the chosen keys and wallets of open vault v.
    LoadChosen(usize),
    /// Choose every key and wallet of open vault v not loaded yet, or
    /// none when all are chosen.
    ChooseAll(usize),
    /// Open or close a Create step.
    CStep(u8),
    /// Close a Create step as done.
    CNext(u8),
    /// Choose or drop a machine the vault will be opened on.
    CMachine(usize),
    /// Choose or drop this computer as one the vault is sized for.
    CHere,
    /// Choose a preset, or `CUSTOM`.
    CPreset(usize),
    /// Choose one of the two sizes, by its place in `SIZES`.
    CSizeRow(usize),
    /// Open the three cards the two sizes stand for: where it opens, the
    /// unlock cost and the space per passphrase.
    CCustomise,
    /// A custom cost's memory.
    CMemory(usize),
    /// A custom cost's passes.
    CPasses(usize),
    /// The slot size.
    CSize(usize),
    /// Another passphrase.
    CAddPhrase,
    /// Remove a passphrase.
    CRemovePhrase(usize),
    /// Type into passphrase n, its first or second field.
    CFocus(usize, bool),
    /// Type the vault's name.
    CName,
    /// Show or hide every passphrase being typed.
    CShow,
    /// Make the vault.
    CGo,
    /// Choose the vault to unlock.
    Pick(usize),
    /// Type the passphrase.
    FocusPassphrase,
    /// Unlock.
    Unlock,
    /// Show an open vault's contents.
    Show(usize),
    /// The selected APK signing key's certificate to the Outbox.
    ApkExport,
    /// Open Sign an APK on the selected key.
    ApkSignOpen,
    /// Look at Inbox APK n before signing it with the selected key.
    ApkPick(usize),
    /// Held to sign the APK looked at.
    ApkHoldSign,
    /// Type the second build's SHA-256.
    ApkSecond,
    /// Back from the review to the list of APKs, or from the list to the
    /// key's page.
    ApkBack,
    /// Sign Inbox APK n with a key of an open vault, from Files.
    ApkFromFiles(usize),
    /// Choose the key, by vault and place, to sign the APK Files offered.
    ApkKeyPick(usize, usize),
    /// A kind in the contents.
    Category(usize),
    /// An item of the kind.
    Item(usize),
    /// Held to show a secret field; does nothing when released.
    Reveal(usize),
    /// Held to delete the item.
    HoldDelete,
    /// Held to delete the files chosen on the Delete files sheet, a vault
    /// among them (`docs/FLOWS.md` decisions 4 and 61).
    HoldWipe,
    /// A key loads at unlock, or stops loading.
    ToggleLoad,
    /// Load the item into the session.
    Load,
    /// Edit the item.
    Edit,
    /// Add an item of the current kind.
    Add,
    /// Add an item of kind n: the contents' row for that kind.
    AddKind(usize),
    /// An empty vault's Write it to a stick: Files.
    WriteOut,
    /// Save session key n into the vault.
    SaveKey(usize),
    /// Save session wallet n into the vault.
    SaveWallet(usize),
    /// Type into the form's field n.
    FocusField(usize),
    /// Keep the form.
    FormSave,
    /// Leave the form.
    FormCancel,
    /// Rename the open vault's slot.
    Rename,
    /// Show what is typed into a passphrase field in the clear, or mask
    /// it again: Unlock's, a prompt's, and an entry's secret fields.
    ShowTyped,
    /// Import the entries in an Inbox text file.
    ImportEntries(usize),
    /// Keep Inbox file n in the open vault as what it is: a text file as a
    /// note, words as a key, a descriptor as a wallet.
    AddFile(usize),
    /// Import the entries in every Inbox entries file.
    ImportAllEntries,
    /// Scan a TOTP setup code into a new entry.
    ScanEntry,
    /// Roll dice for passphrase n.
    Dice(usize),
    /// Choose which of the three EFF lists to roll from, by index into
    /// [`osk_bip::diceware::List::ALL`].
    DiceList(u8),
    /// Fill the passphrase with the rolled words.
    DiceUse,
    /// Put the dice away.
    DiceClose,
    /// Open an Inbox `osk-backup` file into the vault.
    ImportBackup(usize),
    /// The selected entry out as a KeePass database, under a passphrase
    /// asked for first.
    ExportKdbx,
    /// Load the selected key with a BIP-39 passphrase typed now.
    LoadWithPassphrase,
    /// Save session key n into the vault with its BIP-39 passphrase.
    SaveKeyWithPassphrase(usize),
    /// Type into the prompt.
    FocusPrompt,
    /// Do what the prompt asks.
    PromptGo,
    /// Leave the prompt.
    PromptCancel,
    /// A GPG key's expiry, in years; 0 for never.
    GpgYears(u32),
    /// The selected GPG key's public certificate to the Outbox.
    GpgExport,
    /// The revocation certificate kept with the selected GPG key, through
    /// the secret sheet.
    GpgExportRevocation,
    /// The page that asks why the selected GPG key is revoked
    /// (`gpg --generate-revocation`).
    GpgRevocationPage,
    /// The reason chosen on that page, by GnuPG's menu number 0 to 3.
    GpgReason(u8),
    /// The new term chosen for the selected GPG key's renewal, in years;
    /// 0 for never.
    GpgRenewTerm(u32),
    /// Held to mark the selected GPG key revoked.
    GpgHoldRevoke,
    /// New self-signatures with the term chosen.
    GpgRenew,
    /// Choose Inbox files for the selected GPG key to sign.
    GpgSignPick,
    /// Tick or untick Inbox file n to sign.
    GpgTick(usize),
    /// Tick every signable file, or untick them all when every one is
    /// already ticked.
    GpgTickAll,
    /// Sign a file's **Sign**: the sheet over the page with the files
    /// ticked, the time and the hold (F63). Signs nothing.
    GpgSign,
    /// Held on that sheet to sign every ticked file with the selected GPG
    /// key, one signature each to the Outbox.
    GpgHoldSign,
    /// Choose the Windows-compatible enrolment policy, or own keys only.
    SbOwnOnly(bool),
    /// The enrolment files to the Outbox.
    SbEnrol,
    /// Show the Inbox's EFI images, to sign or check.
    SbImages,
    /// Look at image n before signing it.
    SbPick(usize),
    /// Held to sign the image looked at.
    SbHoldSign,
    /// Check image n's signature.
    SbCheck(usize),
    /// On a small panel, back from an item, or what was being added or
    /// made, to the vault's list.
    ItemBack,
    /// Held to show the selected key's SeedQR (`docs/BACKUP-FLOW.md`
    /// §19.7); does nothing when released.
    ShowSeedQr,
    /// The selected wallet's descriptor as the Wallets page shows it.
    WalletQr,
    /// The Load sheet (F62): tick or untick wallet record or Keys-card
    /// key record n.
    LoadTick(usize),
    /// The Load sheet's Unselect all, or Select all once none is ticked.
    LoadTickAll,
    /// The Load sheet's Load ticked.
    LoadTicked,
    /// The Load sheet's Not now: nothing loads, the vault stays open.
    LoadNotNow,
    /// From wallet n short of a key: the seeds open vaults keep without
    /// a passphrase, one to load with a passphrase typed now (F62).
    PassphraseKey(usize),
    /// On that sheet: seed record r of open vault v, its field.
    PassphrasePick(usize, usize),
    /// On that sheet: back from a seed's field to the list.
    PassphraseList,
    /// Save to a stick: the popup of the selected key's or wallet's files.
    SaveToStick,
    /// Tick or untick the popup's file n.
    SaveTick(u8),
    /// The ticked files to the Outbox, for the stick.
    SaveToStickGo,
}

/// Whether an action acts when held long enough rather than on release.
pub fn is_hold(a: Action) -> bool {
    matches!(
        a,
        Action::Vault(
            VaultAction::HoldDelete
                | VaultAction::HoldWipe
                | VaultAction::SbHoldSign
                | VaultAction::GpgHoldSign
                | VaultAction::GpgHoldRevoke
                | VaultAction::ApkHoldSign
        ) | Action::Flow(crate::backup_flow_steps::FlowAction::Hold)
    )
}

/// What a one-line prompt in the contents is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// The passphrase of an Inbox backup, by Inbox position.
    Backup(usize),
    /// A BIP-39 passphrase to load the selected key with.
    KeyPassphrase,
    /// A BIP-39 passphrase for seed record r of open vault v, checked
    /// against the keys a wallet is missing (F62).
    ForWallet(usize, usize),
    /// The passphrase a KeePass export of the selected entry is sealed
    /// under.
    Kdbx,
}

/// A one-line prompt in the contents' detail column.
pub struct Prompt {
    /// What it is for.
    pub purpose: Purpose,
    /// What is typed.
    pub text: TextBox,
    /// Why the last try failed.
    pub error: Option<String>,
}

/// A line of typing, wiped when dropped, cleared or moved.
#[derive(Default)]
pub struct TextBox {
    /// What is typed.
    pub text: crate::secret_text::SecretText,
}

impl TextBox {
    pub(crate) fn clear(&mut self) {
        self.text.clear();
    }
    pub(crate) fn set(&mut self, s: &str) {
        self.text.set(s);
    }
}

/// Where typing goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The unlock passphrase.
    Passphrase,
    /// Create's passphrase n, first or second field.
    Phrase(usize, bool),
    /// The form's field n.
    Field(usize),
    /// The dice rolls.
    Dice,
    /// The prompt.
    Prompt,
    /// Create's name for the vault.
    Name,
    /// Sign an APK's field for the second build's SHA-256.
    Second,
}

/// One vault file the Inbox or Outbox holds.
pub struct VaultFile {
    /// Its file name.
    pub name: String,
    /// It waits in the Outbox (made or sealed here), rather than having
    /// come from a stick.
    pub in_outbox: bool,
    /// Its header.
    pub header: Header,
    /// Its length in bytes.
    pub len: usize,
    /// The open vault it is, when one of its slots is open.
    pub open: Option<usize>,
}

/// A wallet in an open vault that holds some of its seeds too.
#[derive(Debug, Clone)]
pub struct VaultWallet {
    /// The wallet's record.
    pub record: usize,
    /// Its name.
    pub name: String,
    /// The records of its seeds in the same vault.
    pub keys: Vec<usize>,
    /// Signatures it needs.
    pub needed: usize,
    /// Keys it has.
    pub total: usize,
    /// It and those seeds are all loaded.
    pub done: bool,
    /// Chosen for the next load.
    pub chosen: bool,
}

/// A key or wallet in an open vault, as its load list shows it.
#[derive(Debug, Clone)]
pub struct VaultRow {
    /// Its record's place in the slot.
    pub record: usize,
    /// Its label or name.
    pub name: String,
    /// A seed's fingerprint, a wallet's shape.
    pub detail: String,
    /// A seed rather than a wallet.
    pub seed: bool,
    /// The session has it already.
    pub loaded: bool,
    /// Chosen for the next load.
    pub chosen: bool,
}

/// A wallet description a vault record holds, read once
/// ([`VaultRecords::described`]): a wallet record's wallet, a plan's, a
/// wallet share's. Public data only.
#[derive(Debug, Clone)]
pub struct Described {
    /// The description, parsed.
    pub policy: osk_bip::policy::WalletPolicy,
    /// [`crate::wallet::same_wallet`] of it: what tells two descriptions
    /// of one wallet apart from another wallet's.
    pub same: String,
    /// Its descriptor checksum.
    pub checksum: String,
    /// Its shape as the session names one: "Single key", "2 of 3".
    pub shape: String,
}

impl Described {
    /// What is kept of a parsed description.
    fn of(policy: osk_bip::policy::WalletPolicy) -> Described {
        Described {
            same: crate::wallet::same_wallet(&policy),
            checksum: policy.checksum(),
            shape: crate::vault_screens::policy_shape(policy.clone()),
            policy,
        }
    }
}

/// An open vault's records, with the wallet descriptions they hold read
/// once (`docs/SPEED.md` §2 rules 2 and 4, §3.3). Reads as the records
/// ([`Contents`]) themselves.
///
/// Invariant: what [`VaultRecords::described`] answers is what the
/// records hold as they are. Every change to the records goes through
/// `DerefMut`, which drops what was read; the next ask reads them again,
/// and every frame reads them before it draws ([`VaultRecords::warm`]
/// from `render`), so drawing only looks them up.
#[derive(Clone)]
pub struct VaultRecords {
    contents: Contents,
    read: std::cell::OnceCell<Read>,
}

/// What [`VaultRecords`] keeps read of its records, by record place.
#[derive(Clone)]
struct Read {
    /// The wallet description each holds.
    described: Vec<Option<Described>>,
    /// How many words each key record's seed has: public, and none for a
    /// record that holds a seed rather than words.
    words: Vec<Option<usize>>,
    /// Each key record's words' [`crate::wallet::WordsId`]: a hash of
    /// secret entropy, kept only while the vault is open and these
    /// records unchanged, wiped when dropped.
    words_ids: Vec<Option<crate::wallet::WordsId>>,
}

impl VaultRecords {
    /// The records, nothing read yet.
    pub fn new(contents: Contents) -> VaultRecords {
        VaultRecords {
            contents,
            read: std::cell::OnceCell::new(),
        }
    }

    /// Reads the wallet descriptions the records hold, unless they are
    /// read already. Costs a parse of each description the first time
    /// after a change, a lookup after.
    pub fn warm(&self) {
        self.read.get_or_init(|| {
            // Each key record's words, parsed once for both.
            let (words, words_ids) = self
                .contents
                .records
                .iter()
                .map(|r| {
                    let m = (r.kind == kind::KEY)
                        .then(|| crate::wallet::record_words(r))
                        .flatten();
                    (
                        m.as_ref().map(|m| m.word_count()),
                        m.as_ref().map(crate::wallet::words_id),
                    )
                })
                .unzip();
            self.read_with(words, words_ids)
        });
    }

    /// What [`VaultRecords::warm`] keeps, with the key records' words read.
    fn read_with(
        &self,
        words: Vec<Option<usize>>,
        words_ids: Vec<Option<crate::wallet::WordsId>>,
    ) -> Read {
        Read {
            described: self
                .contents
                .records
                .iter()
                .map(|r| {
                    let f = match r.kind {
                        kind::WALLET => field::WALLET,
                        kind::PLAN => field::PLAN_WALLET,
                        kind::WALLET_SHARE => field::SHARE_WALLET,
                        _ => return None,
                    };
                    let policy = crate::wallet::read_wallet(r.text(f)?).ok()?;
                    Some(Described::of(policy))
                })
                .collect(),
            words,
            words_ids,
        }
    }

    /// Key record `i`'s words' [`crate::wallet::WordsId`]; `None` for
    /// another record or one holding a seed rather than words. A lookup
    /// once the records are read.
    pub fn words_id(&self, i: usize) -> Option<&[u8; 32]> {
        self.warm();
        self.read.get()?.words_ids.get(i)?.as_deref()
    }

    /// How many words key record `i`'s seed has; `None` for another
    /// record, or a key record holding a seed rather than words. A lookup
    /// once the records are read.
    pub fn word_count(&self, i: usize) -> Option<usize> {
        self.warm();
        *self.read.get()?.words.get(i)?
    }

    /// The wallet description record `i` holds, read: a wallet record's
    /// wallet, a plan's, a wallet share's; `None` for another record or
    /// one whose description does not read. A lookup once the records
    /// are read ([`VaultRecords::warm`]); the first ask after a change
    /// parses them.
    pub fn described(&self, i: usize) -> Option<&Described> {
        self.warm();
        self.read.get()?.described.get(i)?.as_ref()
    }

    /// The wallet description record `r` holds, read, when `r` is one of
    /// these records itself (told by where it is, not by what it says);
    /// `None` for a copy. A search of the records' places.
    pub fn described_rec(&self, r: &Record) -> Option<&Described> {
        self.described(self.place_of(r)?)
    }

    /// Where record `r` is among these records, when it is one of them
    /// itself (told by where it is, not by what it says).
    pub fn place_of(&self, r: &Record) -> Option<usize> {
        self.contents
            .records
            .iter()
            .position(|x| std::ptr::eq(x, r))
    }

    /// Each record of kind `k` with the wallet description it holds, read
    /// (see [`VaultRecords::described`]).
    pub fn described_of(&self, k: u8) -> impl Iterator<Item = (usize, &Record, &Described)> {
        self.contents
            .of(k)
            .filter_map(move |(i, r)| self.described(i).map(|d| (i, r, d)))
    }
}

impl std::ops::Deref for VaultRecords {
    type Target = Contents;
    fn deref(&self) -> &Contents {
        &self.contents
    }
}

/// A change to the records drops what was read of them: the invariant
/// of [`VaultRecords`].
impl std::ops::DerefMut for VaultRecords {
    fn deref_mut(&mut self) -> &mut Contents {
        self.read = std::cell::OnceCell::new();
        &mut self.contents
    }
}

impl From<Contents> for VaultRecords {
    fn from(contents: Contents) -> VaultRecords {
        VaultRecords::new(contents)
    }
}

impl Faraday {
    /// How many words key record `r`'s seed has, as its open vault keeps
    /// it read ([`VaultRecords::word_count`]); a record that is no open
    /// vault's own (a copy) is read afresh, which costs a parse.
    pub(crate) fn record_word_count(&self, r: &Record) -> Option<usize> {
        if let Some((o, i)) = self
            .vaults
            .open
            .iter()
            .find_map(|o| o.contents.place_of(r).map(|i| (o, i)))
        {
            return o.contents.word_count(i);
        }
        crate::wallet::record_words(r).map(|m| m.word_count())
    }

    /// Key record `r`'s words' [`crate::wallet::WordsId`], as its open
    /// vault keeps it ([`VaultRecords::words_id`]); a record that is no
    /// open vault's own (a copy) is read afresh, which costs a parse.
    pub(crate) fn record_words_id(&self, r: &Record) -> Option<crate::wallet::WordsId> {
        if let Some((o, i)) = self
            .vaults
            .open
            .iter()
            .find_map(|o| o.contents.place_of(r).map(|i| (o, i)))
        {
            return o
                .contents
                .words_id(i)
                .map(|id| zeroize::Zeroizing::new(*id));
        }
        crate::wallet::record_words(r).map(|m| crate::wallet::words_id(&m))
    }

    /// The wallet description record `r` holds, as its open vault keeps
    /// it read ([`VaultRecords`]); a record that is no open vault's own
    /// (a copy) is read afresh, which costs a parse.
    pub(crate) fn record_described(&self, r: &Record) -> Option<std::borrow::Cow<'_, Described>> {
        if let Some(d) = self
            .vaults
            .open
            .iter()
            .find_map(|o| o.contents.described_rec(r))
        {
            return Some(std::borrow::Cow::Borrowed(d));
        }
        let f = match r.kind {
            kind::WALLET => field::WALLET,
            kind::PLAN => field::PLAN_WALLET,
            kind::WALLET_SHARE => field::SHARE_WALLET,
            _ => return None,
        };
        let policy = crate::wallet::read_wallet(r.text(f)?).ok()?;
        Some(std::borrow::Cow::Owned(Described::of(policy)))
    }
}

/// A key record's identity, kept once worked out: the record's id (a
/// hash of its payload and passphrase), its fingerprint, and its master
/// key's [`crate::wallet::master_id`].
pub type KeyIdentity = ([u8; 32], String, [u8; 32]);

/// One open slot of one vault.
pub struct OpenVault {
    /// The file name it came under, and goes back under.
    pub name: String,
    /// The file as it was opened: the other slots are copied from it.
    file: Vec<u8>,
    opened: fv::Opened,
    /// The slot's records, with the wallet descriptions they hold read
    /// once ([`VaultRecords`]).
    pub contents: VaultRecords,
    /// Changes not yet sealed.
    pub changes: usize,
    /// Keys and wallets left out of the next load, by record position.
    pub skip: std::collections::BTreeSet<usize>,
    /// Its seeds and wallets listed one by one on Files, under its
    /// wallets with their seeds.
    pub each_shown: bool,
}

impl OpenVault {
    /// The slot's label, or "Vault".
    pub fn label(&self) -> String {
        self.contents
            .of(kind::SLOT_LABEL)
            .next()
            .and_then(|(_, r)| r.text(field::LABEL))
            .unwrap_or(EN.vaults_vault)
            .to_string()
    }

    /// The header of the file it was opened from.
    pub fn header(&self) -> &Header {
        self.opened.header()
    }
}

/// Create a vault's form.
pub struct CreateForm {
    /// The open step.
    pub open: Option<u8>,
    /// Steps closed as done, by `vstep`.
    pub done: [bool; vstep::COUNT],
    /// Customise has opened the three cards the size stands for.
    pub customise: bool,
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The machines chosen, in `MACHINES` order.
    pub on: [bool; 5],
    /// This computer is not one the vault is sized for: it is chosen
    /// until unticked, for a vault that asks less than it can give.
    pub not_here: bool,
    /// The preset chosen, `CUSTOM` for Custom; `None` takes the
    /// suggestion.
    pub preset: Option<usize>,
    /// A custom cost's memory, MiB.
    pub memory: u32,
    /// A custom cost's passes.
    pub passes: u32,
    /// The slot size.
    pub slot: u32,
    /// The vault's name, which its file is named after.
    pub name: TextBox,
    /// Each passphrase, typed twice.
    pub phrases: Vec<(TextBox, TextBox)>,
    /// For each passphrase the dice made, what they made and its bits:
    /// its strength holds while the field still holds those words.
    pub from_dice: Vec<Option<(zeroize::Zeroizing<String>, f32)>>,
    /// The passphrases are shown in the clear, not masked: a person
    /// cannot accept one on faith, only on seeing it.
    pub shown: bool,
    /// What stops the vault being made.
    pub error: Option<String>,
}

impl Default for CreateForm {
    fn default() -> Self {
        // The size has a default, so the flow opens on Name and
        // passphrases (`DESIGN.md` §4.14).
        let mut done = [false; vstep::COUNT];
        done[usize::from(vstep::PRESET)] = true;
        let (_, preset, on) = SIZES[DEFAULT_SIZE];
        CreateForm {
            open: Some(vstep::PHRASES),
            done,
            customise: false,
            scroll: flow::Scroll::default(),
            on,
            not_here: false,
            preset: Some(preset),
            memory: 512,
            passes: 3,
            slot: fv::DEFAULT_SLOT,
            name: TextBox::default(),
            phrases: vec![(TextBox::default(), TextBox::default())],
            from_dice: vec![None],
            shown: false,
            error: None,
        }
    }
}

impl CreateForm {
    /// The steps shown, in order: the size and the passphrases, or, once
    /// Customise is pressed, the three cards the size stands for in its
    /// place.
    pub fn steps(&self) -> &'static [u8] {
        if self.customise {
            &[vstep::WHERE, vstep::COST, vstep::SIZE, vstep::PHRASES]
        } else {
            &[vstep::PRESET, vstep::PHRASES]
        }
    }
}

/// The form adding or editing an item.
pub struct Form {
    /// The record type.
    pub kind: u8,
    /// The record being edited, by position; `None` adds one.
    pub record: Option<usize>,
    /// The fields: number, label, typing, shown masked.
    pub fields: Vec<(u8, &'static str, TextBox, bool)>,
}

/// Work that waits for a frame saying it is under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    /// Unlock the chosen vault.
    Unlock,
    /// Make the vault the form describes.
    Create,
    /// Time one Argon2id pass over 64 MiB, for the unlock estimates.
    Measure,
    /// Open the backup the prompt names.
    Backup,
    /// Make the key the backup flow's form described.
    FlowMake,
    /// Write the KeePass export the prompt is for.
    Kdbx,
}

/// The app's vaults.
#[derive(Default)]
pub struct Vaults {
    /// The open slots.
    pub open: Vec<OpenVault>,
    /// Create a vault, while it is on screen.
    pub create: Option<CreateForm>,
    /// The vault chosen to unlock, by its place in the list.
    pub pick: usize,
    /// The unlock passphrase.
    pub passphrase: TextBox,
    /// Why the last unlock failed.
    pub unlock_error: Option<String>,
    /// The open vault on the contents screen.
    pub current: usize,
    /// The kind shown.
    pub category: usize,
    /// The kind shown was chosen, by the person or the flow that opened
    /// the contents, rather than set at unlock: it is listed even while it
    /// holds nothing.
    pub category_chosen: bool,
    /// The item shown in each kind.
    pub item: [usize; 7],
    /// On a small panel, the chosen item is the page, not the list.
    pub item_open: bool,
    /// Adding or editing an item.
    pub form: Option<Form>,
    /// Saving session keys or wallets into the vault.
    pub saving: bool,
    /// Where typing goes.
    pub focus: Option<Focus>,
    /// Free memory, MiB, as the shell last said.
    pub memory_free_mib: Option<u32>,
    /// One Argon2id pass over 64 MiB, ms, as measured here.
    pub ms_per_unit: Option<u64>,
    measure_from: Option<u64>,
    /// Work waiting for its frame.
    pub working: Option<Work>,
    /// A frame has been drawn since the work was asked for.
    pub(crate) drawn: bool,
    /// When the press now held began, ms.
    pub(crate) pressed_at: u64,
    /// The vault just made, by file name, until it is unlocked or the
    /// person leaves the list.
    pub just_made: Option<String>,
    /// Where an unlock or a creation started from another flow returns.
    pub back_to: Option<Screen>,
    /// Create a vault was opened from Home: Back goes to Home, not the
    /// list.
    pub from_home: bool,
    /// Dice being rolled for a passphrase: which one, the rolls, and
    /// which of the three EFF lists.
    pub dice: Option<(usize, TextBox, osk_bip::diceware::List)>,
    /// The prompt in the contents.
    pub prompt: Option<Prompt>,
    /// This computer's clock, Unix seconds, as the shell last said.
    pub unix_secs: Option<u64>,
    /// When the shell last said it, on the app's own ticks, ms.
    pub(crate) clock_at_ms: u64,
    /// A time set by hand (`docs/TOTP.md` §1.2): the Unix seconds the
    /// person set minus the shell's clock at that moment. `None` until
    /// the first Set or Keep this time this power-on.
    pub(crate) clock_offset: Option<i64>,
    /// Set or Keep this time has been pressed this power-on
    /// (`docs/TOTP.md` §1.3). The desktop app is always checked, once its
    /// clock is known.
    pub(crate) clock_checked: bool,
    /// What is remembered of each vault seen open since power-on.
    pub summaries: Vec<VaultSummary>,
    /// The expiry chosen for a GPG key being made or renewed, in years;
    /// 0 for never.
    pub gpg_years: u32,
    /// Choosing Inbox files for the selected GPG key to sign.
    pub signing: bool,
    /// The signable Inbox files ticked to sign, by name and SHA-256: a
    /// file replaced by a stick visit is no longer ticked. Set to its
    /// default (the only file ticked, several none) whenever the page
    /// opens.
    pub sign_ticked: Vec<(String, [u8; 32])>,
    /// The fingerprint of the key the signing page was opened for.
    pub sign_key: Option<[u8; 20]>,
    /// The files the signing page hashed, so that it hashes each once.
    pub sign_seen: std::cell::RefCell<Vec<crate::gpg::Seen>>,
    /// The term chosen to renew the selected GPG key for, in years; none
    /// until the person chooses.
    pub renew_years: Option<u32>,
    /// The reason chosen for a revocation, by GnuPG's menu number.
    pub revoke_reason: u8,
    /// What is typed into passphrase fields is shown in the clear.
    pub typed_shown: bool,
    /// APK signing keys: the form, the result, Sign an APK, and the keys
    /// that wait for their copies.
    pub apk: crate::apk::ApkState,
    /// The owner name Secure Boot keys are being made under.
    pub sb_owner: String,
    /// Enrolment adds the owner's keys alone, not Microsoft's.
    pub sb_own_only: bool,
    /// The Inbox's EFI images are shown, to sign or check.
    pub sb_images: bool,
    /// The image being looked at before signing, by Inbox position.
    pub sb_sign: Option<usize>,
    /// What the last check said.
    pub sb_checked: Option<String>,
    /// Key fingerprints worked out for the contents screen, by the
    /// SHA-256 of the key's payload and the passphrase stored with it:
    /// deriving one takes 2048 rounds. In memory only.
    /// With each, the key's [`crate::wallet::master_id`], which says it
    /// is a given key where the fingerprint only finds it.
    pub fingerprints: std::cell::RefCell<Vec<KeyIdentity>>,
    draws: u64,
    /// The selected key's SeedQR while Show SeedQR is held, by record
    /// index: made at the press, dropped at the lift.
    pub(crate) seedqr: Option<(usize, osk_codec::qr::QrMatrix)>,
    /// Save to a stick's popup, while it is open.
    pub stick_save: Option<crate::vault_out::StickSave>,
    /// The Load sheet an unlock shows, while it is up (F62).
    pub load: Option<crate::vault_load::LoadSheet>,
    /// The sheet of seeds kept with no passphrase, while it is up (F62).
    pub pass_seeds: Option<crate::vault_load::PassSeeds>,
}

/// Memory in MiB as a person reads it.
pub fn mib_text(mib: u32) -> String {
    if mib >= 1024 {
        if mib.is_multiple_of(1024) {
            fill(EN.vaults_gib, &[&(mib / 1024).to_string()])
        } else {
            fill(EN.vaults_gib, &[&format!("{:.1}", f64::from(mib) / 1024.0)])
        }
    } else {
        fill(EN.vaults_mib, &[&mib.to_string()])
    }
}

/// A slot size as a person reads it.
pub fn size_text(slot: u32) -> String {
    if slot >= 1_048_576 {
        fill(EN.vaults_mib, &[&(slot / 1_048_576).to_string()])
    } else {
        fill(EN.vaults_kib, &[&(slot / 1024).to_string()])
    }
}

/// A file size as a person reads it.
pub fn file_text(len: usize) -> String {
    if len >= 1_000_000 {
        fill(EN.vaults_mb, &[&format!("{:.1}", len as f64 / 1e6)])
    } else {
        fill(EN.vaults_kb, &[&len.div_ceil(1000).to_string()])
    }
}

/// A cost as a person reads it.
pub fn cost_text(cost: &Cost) -> String {
    format!(
        "{} · {} {}",
        mib_text(cost.memory_kib / 1024),
        cost.passes,
        if cost.passes == 1 {
            EN.vaults_pass
        } else {
            EN.vaults_passes
        }
    )
}

impl Vaults {
    /// Seconds to unlock at a cost on this computer, when measured.
    pub fn seconds(&self, memory_mib: u32, passes: u32) -> Option<f64> {
        self.ms_per_unit
            .map(|ms| ms as f64 * f64::from(memory_mib) / 64.0 * f64::from(passes) / 1000.0)
    }

    /// "about 1.8 s", or that it is not measured yet.
    pub fn time_text(&self, memory_mib: u32, passes: u32) -> String {
        match self.seconds(memory_mib, passes) {
            Some(s) if s < 10.0 => fill(EN.vaults_about_s, &[&format!("{s:.1}")]),
            Some(s) => fill(EN.vaults_about_s, &[&(s.round() as u64).to_string()]),
            None => EN.vaults_not_measured.to_string(),
        }
    }

    /// Create's cost: memory MiB and passes.
    pub fn form_cost(&self) -> Option<(usize, u32, u32)> {
        let c = self.create.as_ref()?;
        let k = c.preset.unwrap_or_else(|| self.suggested());
        Some(match k {
            CUSTOM => (CUSTOM, c.memory, c.passes),
            k => (k, PRESETS[k].1, PRESETS[k].2),
        })
    }

    /// The most unlock memory this computer gives a vault, MiB: the
    /// largest custom memory that fits in what it has free beside twice
    /// the file and the app ([`vault_need_mib`]), at most 4 GiB, which is
    /// as much as a vault needs; 4 GiB when it does not say.
    pub fn here_limit(&self) -> u32 {
        let Some(free) = self.memory_free_mib else {
            return MAX_UNLOCK_MIB;
        };
        let slot = self.create.as_ref().map_or(fv::DEFAULT_SLOT, |c| c.slot);
        CUSTOM_MEMORY
            .iter()
            .copied()
            .filter(|m| *m <= MAX_UNLOCK_MIB && vault_need_mib(*m, slot) <= free)
            .max()
            .unwrap_or(CUSTOM_MEMORY[0])
    }

    /// The machine that gives a vault the least unlock memory among
    /// those chosen, label and MiB: this computer what it has free (see
    /// `here_limit`), any other half of its memory. With none chosen,
    /// this computer.
    pub fn weakest(&self) -> (String, u32) {
        let here = (EN.vaults_this_computer_2.to_string(), self.here_limit());
        let Some(c) = self.create.as_ref() else {
            return here;
        };
        let mut w: Option<(String, u32)> = (!c.not_here).then_some(here.clone());
        for (k, m) in MACHINES.iter().enumerate() {
            if c.on[k] && w.as_ref().is_none_or(|w| m.3 / 2 < w.1) {
                w = Some((m.0.to_string(), m.3 / 2));
            }
        }
        w.unwrap_or(here)
    }

    /// The largest preset the weakest machine gives the memory for.
    pub fn suggested(&self) -> usize {
        let (_, limit) = self.weakest();
        (0..PRESETS.len())
            .rev()
            .find(|&k| PRESETS[k].1 <= limit)
            .unwrap_or(0)
    }

    /// The size row chosen, while the preset and machines are one of
    /// `SIZES`.
    pub fn size_row(&self) -> Option<usize> {
        let c = self.create.as_ref()?;
        SIZES
            .iter()
            .position(|(_, p, on)| c.preset == Some(*p) && c.on == *on && !c.not_here)
    }

    /// Whether a passphrase pair is typed the same twice, and not empty.
    pub fn phrase_ok(c: &CreateForm, i: usize) -> bool {
        let (a, b) = &c.phrases[i];
        !a.text.is_empty() && *a.text == *b.text
    }

    /// Passphrase `i`'s own bits, known only while it is the words the
    /// dice made: a typed passphrase's strength is not measured.
    pub fn phrase_bits(c: &CreateForm, i: usize) -> Option<f32> {
        let (made, bits) = c.from_dice.get(i)?.as_ref()?;
        let (a, b) = c.phrases.get(i)?;
        (**made == *a.text && **made == *b.text).then_some(*bits)
    }
}

impl Faraday {
    /// Puts `phrase` in Create a vault's first passphrase, typed twice, in
    /// place of what is there, for a shell that offers a known passphrase
    /// in place of rolled words; the stick and the desktop never call it.
    pub fn put_passphrase(&mut self, phrase: &str) {
        if let Some(c) = self.vaults.create.as_mut()
            && let Some((a, b)) = c.phrases.get_mut(0)
        {
            a.set(phrase);
            b.set(phrase);
            if let Some(d) = c.from_dice.get_mut(0) {
                *d = None;
            }
            self.dirty = true;
            self.commands.push_back(osk_shell_api::Command::Draw);
        }
    }

    /// Whether anything secret is in memory: a key, or an open vault.
    pub fn holds_secret(&self) -> bool {
        self.session.holds_secret() || !self.vaults.open.is_empty()
    }

    /// The session's keys and wallets that no open vault holds, by label
    /// and by name (`PLAN.md` §6.3: locking lists what was not saved).
    /// Kept in the drawing memo while a frame is drawn (`docs/SPEED.md`
    /// §3.3). Costs a parse of each key's words.
    pub fn unsaved(&self) -> (Vec<String>, Vec<String>) {
        if self.drawing.is_on() {
            return self
                .drawing
                .unsaved
                .get_or_init(|| crate::work::filling(|| self.unsaved_now()))
                .clone();
        }
        self.unsaved_now()
    }

    /// Works out [`Faraday::unsaved`]. Costs a parse of each key's words.
    fn unsaved_now(&self) -> (Vec<String>, Vec<String>) {
        let held_keys: Vec<&[u8]> = self
            .vaults
            .open
            .iter()
            .flat_map(|v| {
                v.contents
                    .of(kind::KEY)
                    .filter_map(|(_, r)| r.field(field::KEY))
            })
            .collect();
        let held_wallets: Vec<String> = self
            .vaults
            .open
            .iter()
            .flat_map(|v| {
                v.contents
                    .described_of(kind::WALLET)
                    .map(|(_, _, d)| d.policy.to_descriptor())
            })
            .collect();
        let keys = self
            .session
            .keys
            .iter()
            .filter(|k| {
                let payload = k
                    .words
                    .as_ref()
                    .and_then(|w| crate::wallet::parse_words(k.language, w).ok())
                    .map(|m| records::words_payload(&m));
                !payload.is_some_and(|p| held_keys.contains(&p.as_slice()))
            })
            .map(|k| k.label.clone())
            .collect();
        let wallets = self
            .session
            .wallets
            .iter()
            .filter(|w| !held_wallets.contains(&w.policy().to_descriptor()))
            .map(|w| w.name.clone())
            .collect();
        (keys, wallets)
    }

    /// What a lock, a write-out or a power-off keeps and what it wipes
    /// (`docs/SIMPLIFY.md` §4.5): each vault with its currency, the files
    /// For the stick, and each seed and wallet in memory with where else
    /// it is. At `power_off` a vault not written as it stands and the
    /// files For the stick are lost, not kept. Reads each seed's words and
    /// each open vault's wallets; kept for the frame while one is drawn
    /// (`docs/SPEED.md` §3.12).
    pub fn kept_wiped(&self, power_off: bool) -> KeptWiped {
        let on = self.drawing.is_on();
        if on
            && let Some((_, kept)) = self
                .drawing
                .kept_wiped
                .borrow()
                .iter()
                .find(|(p, _)| *p == power_off)
        {
            return kept.clone();
        }
        let out = crate::work::filling(|| self.kept_wiped_now(power_off));
        if on {
            self.drawing
                .kept_wiped
                .borrow_mut()
                .push((power_off, out.clone()));
        }
        out
    }

    /// Reads each seed's words and each open vault's wallets: what
    /// [`Self::kept_wiped`] answers.
    fn kept_wiped_now(&self, power_off: bool) -> KeptWiped {
        let mut out = KeptWiped::default();
        // Make a new wallet before its keys are kept anywhere ends with
        // the lock or the power-off (`docs/NEW-WALLET.md` §18).
        if self.lock_held() {
            out.wiped.push((EN.lock_held_ends.to_string(), true));
        }
        if power_off {
            for line in self.backup_power_lines() {
                out.wiped.push((line, true));
            }
        }
        for f in self.vault_files() {
            let open = f.open.and_then(|o| self.vaults.open.get(o));
            let title = open
                .map(|v| v.label())
                .or_else(|| self.vault_summary(&f).map(|s| s.name.clone()))
                .unwrap_or_else(|| f.name.clone());
            let currency = self.currency(&f);
            let state = currency
                .as_ref()
                .map_or_else(|| self.medium.from_box().to_string(), Currency::line);
            let line = format!("{title} · {state}");
            let lost =
                power_off && matches!(currency, Some(Currency::NeverWritten | Currency::Changed));
            // A lock with no seed to seal with loses the changes.
            let unsealed = !power_off && !self.seeded() && open.is_some_and(|v| v.changes > 0);
            if unsealed {
                out.wiped
                    .push((format!("{title} · {}", EN.vaults_changes_not_sealed), true));
            } else if lost {
                out.wiped.push((line, true));
            } else {
                out.kept.push(line);
            }
        }
        let n = self.outbox.len();
        let files = format!(
            "{n} {} {}",
            if n == 1 {
                EN.vaults_file
            } else {
                EN.vaults_files
            },
            self.medium.for_the()
        );
        if power_off {
            if n > 0 {
                out.wiped.push((files, true));
            }
        } else {
            out.kept.push(files);
        }
        // Where an open vault holds each seed and wallet.
        // A seed is in a vault when a key record there holds its words,
        // from any list: told by the hash of their entropy each keeps
        // (`Key::words_id`, `VaultRecords::words_id`), not parsed here.
        let in_vault_key = |id: &[u8; 32]| {
            self.vaults.open.iter().find_map(|v| {
                v.contents
                    .of(kind::KEY)
                    .any(|(i, _)| v.contents.words_id(i) == Some(id))
                    .then(|| v.label())
            })
        };
        let in_vault_wallet = |descriptor: &str| {
            self.vaults.open.iter().find_map(|v| {
                v.contents
                    .described_of(kind::WALLET)
                    .any(|(_, _, d)| d.policy.to_descriptor() == descriptor)
                    .then(|| v.label())
            })
        };
        for k in &self.session.keys {
            let vault = k.words_id().and_then(in_vault_key);
            let copy = self.seed_copy_checked(k.fingerprint().0);
            let (state, nowhere) = match (vault, copy) {
                (Some(v), true) => (fill(EN.vaults_in_copy_checked, &[&v.to_string()]), false),
                (Some(v), false) => (fill(EN.vaults_in, &[&v.to_string()]), false),
                (None, true) => (EN.vaults_copy_checked.to_string(), false),
                (None, false) => (EN.vaults_nowhere_else.to_string(), true),
            };
            // A key with no name of its own goes by "Seed {fingerprint}",
            // as the backup map writes it.
            let fp = crate::wallet::fp_text(k.fingerprint());
            let name = if k.label.is_empty() || k.label.eq_ignore_ascii_case(&fp) {
                fill(EN.vaults_seed_n, &[&fp.to_string()])
            } else {
                k.label.clone()
            };
            out.wiped.push((format!("{name} · {state}"), nowhere));
        }
        for w in &self.session.wallets {
            let in_files = self
                .inbox
                .iter()
                .chain(self.outbox.iter())
                .any(|i| i.name == w.source);
            let (state, nowhere) = match in_vault_wallet(&w.policy().to_descriptor()) {
                Some(v) => (fill(EN.vaults_in, &[&v.to_string()]), false),
                None if in_files => (EN.vaults_in_files.to_string(), false),
                None => (EN.vaults_nowhere_else.to_string(), true),
            };
            out.wiped.push((format!("{} · {state}", w.name), nowhere));
        }
        // What is nowhere else first, where it is read.
        out.wiped.sort_by_key(|(_, nowhere)| !nowhere);
        out
    }

    /// The vaults the Inbox and Outbox hold: one row per vault, the
    /// Outbox's copy standing for a vault that is in both.
    pub fn vault_files(&self) -> Vec<VaultFile> {
        let mut out: Vec<VaultFile> = Vec::new();
        for (in_outbox, items) in [(true, &self.outbox), (false, &self.inbox)] {
            for it in items.iter().filter(|i| i.kind == FileKind::Vault) {
                let Ok(header) = fv::read_header(&it.bytes) else {
                    continue;
                };
                if out.iter().any(|v| v.header.salt == header.salt) {
                    continue;
                }
                let open = self
                    .vaults
                    .open
                    .iter()
                    .position(|o| o.header().salt == header.salt);
                out.push(VaultFile {
                    name: it.name.clone(),
                    in_outbox,
                    header,
                    len: it.bytes.len(),
                    open,
                });
            }
        }
        // Open vaults whose file is in neither box still list.
        out
    }

    /// What a flow that needs a vault offers while none is open, coming
    /// back to `back`: Make a vault when there is no vault file, Unlock
    /// {name} when one is locked, Unlock a vault, through the list, when
    /// several are. `None` while a vault is open.
    pub fn vault_way(&self, back: Screen) -> Option<(String, crate::Action)> {
        if self.vaults.open.get(self.vaults.current).is_some() {
            return None;
        }
        Some(self.vault_unlock_way(back).unwrap_or_else(|| {
            (
                EN.vaults_make_a_vault.to_string(),
                crate::Action::Vault(VaultAction::CreateFrom(back)),
            )
        }))
    }

    /// The way to a vault already on a stick read this power-on, whether
    /// or not another is open: Unlock {name} for the one locked, Unlock a
    /// vault, through the list, for several. `None` with none locked.
    pub fn vault_unlock_way(&self, back: Screen) -> Option<(String, crate::Action)> {
        let files = self.vault_files();
        let locked: Vec<usize> = (0..files.len())
            .filter(|&i| files[i].open.is_none())
            .collect();
        let (label, a) = match locked[..] {
            [] => return None,
            [k] => (
                fill(EN.vaults_unlock, &[&files[k].name.to_string()]),
                VaultAction::OpenFrom(k, back),
            ),
            _ => (
                EN.vaults_unlock_a_vault_2.to_string(),
                VaultAction::ListFrom(back),
            ),
        };
        Some((label, crate::Action::Vault(a)))
    }

    /// The flow a vault is being made or unlocked for, by name: the way
    /// back's label and the "Then:" on Create a vault.
    pub fn back_name(&self, s: Screen) -> &'static str {
        match s {
            Screen::Create => EN.vaults_make_a_new_wallet,
            Screen::Restore => EN.vaults_restore_an_existing,
            Screen::Family => EN.vaults_spend,
            Screen::Backup => EN.vaults_back_up_a_wallet,
            Screen::BackupFlow => self
                .flow()
                .map_or(crate::strings::EN.backup_flow_heading, |f| f.kind.name()),
            Screen::Bip85 => EN.vaults_child_seeds,
            Screen::Lightning => EN.vaults_lightning_node,
            Screen::Home if self.import.is_some() => EN.vaults_import,
            Screen::VaultContents => crate::catalog::TILES
                .iter()
                .find(|t| t.go == crate::catalog::Go::VaultCategory(self.vaults.category))
                .map_or(
                    CATEGORIES[self.vaults.category.min(CATEGORIES.len() - 1)].0,
                    |t| t.name,
                ),
            _ => crate::compact::screen_name(s, self.medium),
        }
    }

    /// Where the way back from making or unlocking a vault for `s` goes,
    /// by name: Tools for a vault category with no vault open, as
    /// [`VaultAction::Back`] goes there; else the flow.
    pub fn back_link_name(&self, s: Screen) -> &'static str {
        if s == Screen::VaultContents && self.vaults.open.get(self.vaults.current).is_none() {
            EN.vaults_tools
        } else {
            self.back_name(s)
        }
    }

    /// The secret sheet closes while a vault is made or unlocked for it;
    /// its secret is kept, and the sheet opens again when the vault does.
    fn secret_sheet_waits(&mut self) {
        if self.sheet == Some(crate::Sheet::SecretOut) {
            self.sheet = None;
        }
    }

    fn vault_bytes(&self, f: &VaultFile) -> Option<Vec<u8>> {
        let items = if f.in_outbox {
            &self.outbox
        } else {
            &self.inbox
        };
        items
            .iter()
            .find(|i| i.name == f.name)
            .map(|i| i.bytes.clone())
    }

    /// 32 bytes for a vault's seal nonce: the session's seed, hashed with
    /// what they are for and a count of the draws, so no two seals of a
    /// process share a nonce and two processes share one only if the
    /// system gave both the same bytes. A seal at lock does not wait for
    /// the system beyond its first answer; every key comes from
    /// [`Faraday::fresh`] instead. `None` before the seed arrived: nothing
    /// is sealed then, and the shell is asked again.
    pub(crate) fn vault_draw(&mut self, what: &[u8]) -> Option<[u8; 32]> {
        let Some(seed) = self.seed.as_ref() else {
            self.commands
                .push_back(osk_shell_api::Command::RequestEntropy);
            return None;
        };
        self.vaults.draws += 1;
        let what = [b"faraday vault draw".as_slice(), what].concat();
        Some(seed.draw(&what, &self.vaults.draws.to_le_bytes()))
    }

    /// Runs a vault action.
    pub(crate) fn vault_act(&mut self, a: VaultAction) {
        use VaultAction as V;
        // A small panel's contents page changes to another page: it starts
        // at its top.
        if matches!(
            a,
            V::Item(_)
                | V::ItemBack
                | V::Add
                | V::AddKind(_)
                | V::Edit
                | V::Rename
                | V::Category(_)
        ) {
            self.list_offset = 0.0;
        }
        match a {
            V::Open(i) => {
                let files = self.vault_files();
                if let Some(f) = files.get(i) {
                    match f.open {
                        Some(o) => {
                            self.vaults.current = o;
                            self.vaults.item_open = false;
                            self.screen = Screen::VaultContents;
                        }
                        None => {
                            self.vaults.pick = i;
                            self.vaults.unlock_error = None;
                            self.vaults.focus = self.may_load_keys().then_some(Focus::Passphrase);
                            self.screen = Screen::Unlock;
                            // The import sheet's way back is set afresh by
                            // `OpenFrom` below each time; one left from
                            // an Unlock abandoned by the sidebar does not
                            // linger. Create's and Restore's stay: a vault
                            // just made there is opened through here.
                            if self.vaults.back_to == Some(Screen::Home) {
                                self.vaults.back_to = None;
                            }
                            self.vault_measure();
                        }
                    }
                }
            }
            V::Create | V::CreateFrom(_) => {
                self.vaults.back_to = match a {
                    V::CreateFrom(s) => Some(s),
                    _ => None,
                };
                self.vaults.from_home = self.screen == Screen::Home;
                self.secret_sheet_waits();
                self.vaults.create = Some(CreateForm::default());
                self.vaults.just_made = None;
                // It opens on Name and passphrases: the first passphrase
                // takes the typing, as Continue into that card does.
                self.vaults.focus = self.may_load_keys().then_some(Focus::Phrase(0, false));
                self.screen = Screen::CreateVault;
                self.vault_measure();
            }
            V::OpenFrom(i, back) => {
                self.vault_act(V::Open(i));
                if self.screen == Screen::Unlock {
                    self.vaults.back_to = Some(back);
                    self.secret_sheet_waits();
                }
            }
            V::ListFrom(back) => {
                self.vaults.back_to = Some(back);
                self.vaults.just_made = None;
                self.secret_sheet_waits();
                self.screen = Screen::Vaults;
            }
            V::EachShown(v) => {
                if let Some(o) = self.vaults.open.get_mut(v) {
                    o.each_shown = !o.each_shown;
                }
            }
            V::LoadKeyOf(fp) => {
                let fp = osk_bip::keys::Fingerprint(fp);
                if let Some((v, i)) = self.vault_key_for(fp) {
                    let (k, _) = self.vault_load_set(v, &BTreeSet::from([i]));
                    if k > 0 {
                        self.toast(&fill(
                            EN.vaults_loaded_from,
                            &[
                                &crate::wallet::fp_text(fp).to_string(),
                                &self.vaults.open[v].name.to_string(),
                            ],
                        ));
                    }
                }
            }
            V::LoadWithKeys(v) => {
                let mut set = BTreeSet::new();
                let open: Vec<VaultWallet> = self
                    .vault_wallets_with_keys(v)
                    .into_iter()
                    .filter(|w| !w.done)
                    .collect();
                // One wallet has no checkbox: the button is its own.
                let one = open.len() == 1;
                for w in open {
                    if w.chosen || one {
                        set.insert(w.record);
                        set.extend(w.keys);
                    }
                }
                let (keys, wallets) = self.vault_load_set(v, &set);
                if self.screen == Screen::Family {
                    self.family_settle();
                    self.family_vault_loaded();
                    self.family_opened();
                }
                self.toast(&fill(
                    EN.vaults_and_loaded,
                    &[
                        &wallets.to_string(),
                        (if wallets == 1 {
                            EN.vaults_wallet_inline
                        } else {
                            EN.vaults_wallets_inline
                        }),
                        &keys.to_string(),
                        (if keys == 1 {
                            EN.vaults_seed_inline
                        } else {
                            EN.vaults_seeds
                        }),
                    ],
                ));
            }
            V::OpenWalletOf(v, r) => {
                if let Some(w) = self.vault_load_wallet(v, r) {
                    self.act(Action::OpenWallet(w));
                }
            }
            V::ChecklistOf(v, r) => {
                if let Some(w) = self.vault_load_wallet(v, r) {
                    self.act(Action::BackupChecklist(w));
                }
            }
            V::Back => {
                self.vaults.just_made = None;
                self.flow_create_left();
                if let Some(s) = self.vaults.back_to.take() {
                    self.vaults.passphrase.clear();
                    self.vaults.typed_shown = false;
                    self.vaults.unlock_error = None;
                    self.vaults.create = None;
                    self.vaults.dice = None;
                    self.vaults.focus = None;
                    // The way back from making or unlocking a vault for a
                    // secret is its Cancel: the secret is dropped.
                    self.secret_cancel();
                    // A vault category from Tools with no vault open has
                    // nothing to show: back to Tools.
                    self.screen = if s == Screen::VaultContents
                        && self.vaults.open.get(self.vaults.current).is_none()
                    {
                        Screen::Catalog
                    } else {
                        s
                    };
                }
            }
            V::Choose(v, r) => {
                if let Some(o) = self.vaults.open.get_mut(v)
                    && !o.skip.remove(&r)
                {
                    o.skip.insert(r);
                }
            }
            V::ChooseAll(v) => {
                let rows = self.vault_rows(v);
                if let Some(o) = self.vaults.open.get_mut(v) {
                    let open: Vec<usize> = rows
                        .iter()
                        .filter(|r| !r.loaded)
                        .map(|r| r.record)
                        .collect();
                    if open.iter().all(|r| !o.skip.contains(r)) {
                        o.skip.extend(open);
                    } else {
                        for r in open {
                            o.skip.remove(&r);
                        }
                    }
                }
            }
            V::LoadAll(v) | V::LoadChosen(v) => {
                let all = matches!(a, V::LoadAll(_));
                let (keys, wallets) = self.vault_load(v, all);
                self.wallet = 0;
                // The list stays on screen with what loaded marked; the
                // Spend tab goes on to what it needs next.
                if self.screen == Screen::Family {
                    self.family_settle();
                    self.family_vault_loaded();
                    self.family_opened();
                }
                self.toast(&fill(
                    EN.vaults_and_loaded,
                    &[
                        &keys.to_string(),
                        (if keys == 1 {
                            EN.vaults_seed_inline
                        } else {
                            EN.vaults_seeds
                        }),
                        &wallets.to_string(),
                        (if wallets == 1 {
                            EN.vaults_wallet_inline
                        } else {
                            EN.vaults_wallets_inline
                        }),
                    ],
                ));
            }
            V::CStep(k) => {
                if let Some(c) = self.vaults.create.as_mut() {
                    c.open = if c.open == Some(k) { None } else { Some(k) };
                    c.scroll.follow = true;
                    if c.open == Some(vstep::PHRASES) {
                        self.vaults.focus = Some(Focus::Phrase(0, false));
                    } else if matches!(self.vaults.focus, Some(Focus::Phrase(..) | Focus::Name)) {
                        // Typing goes nowhere on a card with no field.
                        self.vaults.focus = None;
                    }
                }
            }
            V::CNext(k) => {
                if let Some(c) = self.vaults.create.as_mut()
                    && usize::from(k) < vstep::COUNT
                {
                    c.done[k as usize] = true;
                    c.open = c.steps().iter().copied().find(|&i| !c.done[i as usize]);
                    c.scroll.follow = true;
                    c.error = None;
                    if c.open == Some(vstep::PHRASES) {
                        self.vaults.focus = Some(Focus::Phrase(0, false));
                    }
                }
            }
            V::CHere => {
                if let Some(c) = self.vaults.create.as_mut() {
                    c.not_here = !c.not_here;
                    c.error = None;
                }
            }
            V::CMachine(k) => {
                if let Some(c) = self.vaults.create.as_mut()
                    && k < MACHINES.len()
                {
                    c.on[k] = !c.on[k];
                    c.error = None;
                }
            }
            V::CPreset(k) => {
                if let Some(c) = self.vaults.create.as_mut() {
                    c.preset = Some(k.min(CUSTOM));
                    c.error = None;
                }
            }
            V::CSizeRow(k) => {
                if let Some(c) = self.vaults.create.as_mut()
                    && let Some(&(_, preset, on)) = SIZES.get(k)
                {
                    c.preset = Some(preset);
                    c.on = on;
                    c.not_here = false;
                    c.error = None;
                }
            }
            V::CCustomise => {
                if let Some(c) = self.vaults.create.as_mut() {
                    // The three cards open on the size's values, to be
                    // walked through in order.
                    c.customise = true;
                    for k in [vstep::WHERE, vstep::COST, vstep::SIZE] {
                        c.done[usize::from(k)] = false;
                    }
                    if matches!(self.vaults.focus, Some(Focus::Phrase(..) | Focus::Name)) {
                        self.vaults.focus = None;
                    }
                    c.open = Some(vstep::WHERE);
                    c.scroll.follow = true;
                    c.error = None;
                }
            }
            V::CMemory(k) => {
                if let Some(c) = self.vaults.create.as_mut() {
                    c.memory = CUSTOM_MEMORY[k.min(CUSTOM_MEMORY.len() - 1)];
                    c.preset = Some(CUSTOM);
                }
            }
            V::CPasses(k) => {
                if let Some(c) = self.vaults.create.as_mut() {
                    c.passes = CUSTOM_PASSES[k.min(CUSTOM_PASSES.len() - 1)];
                    c.preset = Some(CUSTOM);
                }
            }
            V::CSize(k) => {
                if let Some(c) = self.vaults.create.as_mut() {
                    c.slot = fv::SLOT_SIZES[k.min(3)];
                }
            }
            V::CAddPhrase => {
                if let Some(c) = self.vaults.create.as_mut()
                    && c.phrases.len() < fv::SLOTS
                {
                    c.phrases.push((TextBox::default(), TextBox::default()));
                    c.from_dice.push(None);
                    self.vaults.focus = Some(Focus::Phrase(c.phrases.len() - 1, false));
                }
            }
            V::CRemovePhrase(i) => {
                if let Some(c) = self.vaults.create.as_mut()
                    && c.phrases.len() > 1
                    && i < c.phrases.len()
                {
                    c.phrases.remove(i);
                    if i < c.from_dice.len() {
                        c.from_dice.remove(i);
                    }
                    self.vaults.focus = None;
                }
            }
            V::CFocus(i, second) => {
                self.vaults.focus = self.may_load_keys().then_some(Focus::Phrase(i, second));
            }
            V::CName => self.vaults.focus = Some(Focus::Name),
            V::CShow => {
                if let Some(c) = self.vaults.create.as_mut() {
                    c.shown = !c.shown;
                }
            }
            V::CGo => self.vault_create_check(),
            V::Pick(i) => {
                self.vaults.pick = i;
                self.vaults.unlock_error = None;
                self.vaults.focus = self.may_load_keys().then_some(Focus::Passphrase);
            }
            V::FocusPassphrase => {
                self.vaults.focus = self.may_load_keys().then_some(Focus::Passphrase);
            }
            V::Unlock => self.vault_unlock_check(),
            V::Show(i) => {
                self.gpg_page_reset();
                if i < self.vaults.open.len() {
                    self.vaults.current = i;
                    self.vaults.apk.leave();
                    self.vaults.form = None;
                    self.vaults.saving = false;
                    self.vaults.item_open = false;
                    self.vault_item_offset = 0.0;
                }
            }
            V::Category(k) if !self.category_listed(k) => {}
            V::AddKind(k) if !self.category_listed(k) => {}
            V::ApkSignOpen | V::ApkFromFiles(_) if !self.category_listed(APK_CATEGORY) => {}
            V::Category(k) => {
                self.vaults.apk.leave();
                self.gpg_page_reset();
                self.vaults.category = k.min(CATEGORIES.len() - 1);
                self.vault_item_offset = 0.0;
                self.vaults.category_chosen = true;
                self.vaults.form = None;
                self.vaults.saving = false;
                self.vaults.item_open = false;
            }
            V::Item(k) => {
                self.vaults.apk.leave();
                self.gpg_page_reset();
                self.vaults.item[self.vaults.category] = k;
                self.vault_item_offset = 0.0;
                self.vaults.form = None;
                self.vaults.saving = false;
                self.vaults.item_open = true;
                // A key made before revocations were kept gets its one
                // now, on first use of its page.
                self.gpg_keep_revocation();
            }
            V::ItemBack => {
                self.gpg_page_reset();
                self.vaults.item_open = false;
                self.vaults.form = None;
                self.vaults.prompt = None;
                self.vaults.saving = false;
                self.vaults.sb_images = false;
                self.vaults.sb_sign = None;
                self.vaults.apk.leave();
                self.vaults.focus = None;
            }
            V::Reveal(_) | V::ShowSeedQr => {}
            V::WalletQr => self.vault_wallet_qr(),
            V::SaveToStick => self.vault_stick_save_open(),
            V::SaveTick(k) => {
                if let Some(f) = self
                    .vaults
                    .stick_save
                    .as_mut()
                    .and_then(|s| s.files.get_mut(usize::from(k)))
                {
                    f.ticked = !f.ticked;
                }
            }
            V::SaveToStickGo => self.vault_stick_save_write(),
            V::LoadTick(_) | V::LoadTickAll | V::LoadTicked | V::LoadNotNow => {
                self.load_sheet_act(a)
            }
            V::PassphraseKey(w) => self.pass_seeds_open(w),
            V::PassphrasePick(v, r) => self.pass_seed_pick(v, r),
            V::PassphraseList => self.pass_seed_list(),
            V::HoldDelete => self.vault_delete(),
            V::HoldWipe => self.delete_go(),
            V::ToggleLoad => {
                // A key that loads with a wallet of this vault has no
                // setting of its own (F62).
                let Some(at) = self.vault_selected_index() else {
                    return;
                };
                if self.vault_key_wallet(self.vaults.current, at).is_some() {
                    return;
                }
                if let Some(v) = self.vaults.open.get_mut(self.vaults.current)
                    && let Some(r) = v.contents.records.get_mut(at)
                    && matches!(r.kind, kind::KEY | kind::WALLET)
                {
                    let loads = records::loads_at_unlock(r);
                    records::set_loads_at_unlock(r, !loads);
                    v.changes += 1;
                }
            }
            V::Load => self.vault_load_selected(),
            V::Edit => self.vault_form(true),
            V::AddKind(k) => {
                self.vault_act(V::Category(k));
                self.vault_act(V::Add);
            }
            V::WriteOut => self.screen = Screen::Files,
            V::Add => {
                let kinds = CATEGORIES[self.vaults.category].1;
                // A key that cannot be made again from seed words is made
                // by the backup flow, which decides where it will be kept
                // first (`docs/BACKUP-FLOW.md`).
                if let Some(k) = crate::backup_flow::Kind::of_category(self.vaults.category) {
                    self.vaults.form = None;
                    return self.flow_begin(k);
                }
                self.vaults.sb_images = false;
                self.vaults.sb_sign = None;
                self.vaults.apk.leave();
                if kinds.contains(&kind::KEY) || kinds.contains(&kind::WALLET) {
                    self.vaults.saving = !self.vaults.saving;
                    self.vaults.form = None;
                } else {
                    self.vault_form(false);
                }
            }
            V::SaveKey(k) => self.vault_save_key(k),
            V::SaveWallet(k) => self.vault_save_wallet(k),
            V::FocusField(k) => self.vaults.focus = Some(Focus::Field(k)),
            V::FormSave => self.vault_form_save(),
            V::FormCancel => {
                self.vaults.form = None;
                self.vaults.focus = None;
            }
            V::Rename => {
                let label = self
                    .vaults
                    .open
                    .get(self.vaults.current)
                    .map(OpenVault::label);
                if let Some(label) = label {
                    let mut name = TextBox::default();
                    name.set(&label);
                    self.vaults.form = Some(Form {
                        kind: kind::SLOT_LABEL,
                        record: None,
                        fields: vec![(field::LABEL, EN.vaults_name, name, false)],
                    });
                    self.vaults.saving = false;
                    self.vaults.focus = Some(Focus::Field(0));
                }
            }
            V::ShowTyped => self.vaults.typed_shown = !self.vaults.typed_shown,
            V::ImportEntries(k) => self.vault_import_entries(&[k]),
            V::AddFile(k) => self.vault_add_file(k),
            V::ImportAllEntries => {
                let all: Vec<usize> = self
                    .inbox
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.kind == FileKind::Entries)
                    .map(|(k, _)| k)
                    .collect();
                self.vault_import_entries(&all);
            }
            V::ScanEntry => {
                let mut scan = crate::ScanState::default();
                scan.purpose = crate::ScanPurpose::VaultEntry;
                self.scan = Some(scan);
                self.sheet = Some(crate::Sheet::Scan);
                self.commands.push_back(osk_shell_api::Command::CameraOn);
            }
            V::Dice(_) if !self.may_load_keys() => {}
            V::Dice(i) => {
                // Pressed again, the rolls box keeps what is rolled.
                if self.vaults.dice.as_ref().is_none_or(|d| d.0 != i) {
                    self.vaults.dice =
                        Some((i, TextBox::default(), osk_bip::diceware::List::Large));
                }
                self.vaults.focus = Some(Focus::Dice);
            }
            V::DiceList(k) => {
                if let Some((_, rolls, list)) = self.vaults.dice.as_mut()
                    && let Some(&l) = osk_bip::diceware::List::ALL.get(usize::from(k))
                    && l != *list
                {
                    *list = l;
                    rolls.clear();
                }
            }
            V::DiceUse => {
                if let Some((i, rolls, list)) = self.vaults.dice.take() {
                    let words = dice_words(&rolls.text, list);
                    // The demo puts its public passphrase in place of the
                    // rolled words, which are dropped (`docs/DEMO.md` §5.2).
                    let public = !self.makes_secrets();
                    if let Some(c) = self.vaults.create.as_mut()
                        && let Some((a, b)) = c.phrases.get_mut(i)
                        && !words.is_empty()
                    {
                        let (phrase, bits) = if public {
                            (
                                Zeroizing::new(crate::demo::PASSPHRASE.to_string()),
                                crate::demo::PASSPHRASE_WORDS as f32
                                    * osk_bip::diceware::List::Large.bits_per_word(),
                            )
                        } else {
                            (
                                Zeroizing::new(words.join("")),
                                words.len() as f32 * list.bits_per_word(),
                            )
                        };
                        a.set(&phrase);
                        b.set(&phrase);
                        if let Some(d) = c.from_dice.get_mut(i) {
                            *d = Some((phrase, bits));
                        }
                    }
                    self.vaults.focus = None;
                }
            }
            V::DiceClose => {
                self.vaults.dice = None;
                self.vaults.focus = None;
            }
            V::ImportBackup(k) => {
                if self.vaults.open.is_empty() {
                    self.toast(EN.vaults_unlock_a_vault);
                } else {
                    self.vaults.prompt = Some(Prompt {
                        purpose: Purpose::Backup(k),
                        text: TextBox::default(),
                        error: None,
                    });
                    self.vaults.form = None;
                    self.vaults.saving = false;
                    self.vaults.focus = Some(Focus::Prompt);
                    self.screen = Screen::VaultContents;
                }
            }
            V::LoadWithPassphrase => {
                self.vaults.prompt = Some(Prompt {
                    purpose: Purpose::KeyPassphrase,
                    text: TextBox::default(),
                    error: None,
                });
                self.vaults.focus = Some(Focus::Prompt);
            }
            V::SaveKeyWithPassphrase(k) => self.vault_save_key_with(k, true),
            V::FocusPrompt => self.vaults.focus = Some(Focus::Prompt),
            V::PromptGo => self.vault_prompt_go(),
            V::ExportKdbx => {
                self.vaults.prompt = Some(Prompt {
                    purpose: Purpose::Kdbx,
                    text: TextBox::default(),
                    error: None,
                });
                self.vaults.focus = Some(Focus::Prompt);
            }
            V::PromptCancel => {
                self.vaults.prompt = None;
                self.vaults.focus = None;
            }
            // A key made in the backup flow signs nothing until every place
            // of its plan is written: the action opens the flow.
            V::GpgSignPick
            | V::GpgSign
            | V::GpgHoldSign
            | V::GpgRenew
            | V::SbEnrol
            | V::SbPick(_)
            | V::SbHoldSign
                if self.selected_waits().is_some() =>
            {
                self.flow_resume_selected();
            }
            V::GpgYears(y) => self.vaults.gpg_years = y,
            V::GpgExport => self.gpg_export(),
            V::GpgExportRevocation => self.gpg_export_revocation(),
            V::GpgRevocationPage => {
                self.gpg_keep_revocation();
                self.vault_form_for_revocation();
            }
            V::GpgReason(n) => self.vaults.revoke_reason = n.min(3),
            V::GpgRenewTerm(y) => self.vaults.renew_years = Some(y),
            V::GpgHoldRevoke => self.gpg_mark_revoked(),
            V::GpgRenew => {
                if !self.time_asked() {
                    self.gpg_renew();
                }
            }
            V::GpgSignPick => {
                self.vaults.signing = !self.vaults.signing;
                self.vaults.form = None;
                self.vaults.sign_key = self.gpg_fingerprint();
                if self.vaults.signing {
                    self.gpg_default_ticks();
                } else {
                    self.vaults.sign_ticked.clear();
                }
            }
            V::GpgTick(k) => self.gpg_tick(k),
            V::GpgTickAll => self.gpg_tick_all(),
            // Sign opens the sheet; only its hold signs, and it reads the
            // time the sheet shows rather than asking for it (F63).
            V::GpgSign => self.gpg_sign_sheet_open(),
            V::GpgHoldSign => self.gpg_sign_sheet_hold(),
            V::SbOwnOnly(on) => self.vaults.sb_own_only = on,
            V::SbEnrol => {
                if !self.time_asked() {
                    self.sb_enrol();
                }
            }
            V::SbImages => {
                self.vaults.sb_images = !self.vaults.sb_images;
                self.vaults.sb_sign = None;
                self.vaults.sb_checked = None;
            }
            V::SbPick(k) => self.vaults.sb_sign = Some(k),
            V::SbHoldSign => {
                if let Some(k) = self.vaults.sb_sign {
                    self.sb_sign(k);
                }
            }
            V::SbCheck(k) => self.sb_check(k),
            V::ApkExport => self.apk_export(),
            V::ApkSignOpen => {
                if let Some((_, r)) = self.apk_selected()
                    && let Some(fp) = crate::apk::fingerprint_of(&r)
                    && self
                        .backup_waits(crate::backup_flow::Kind::Apk, &fp)
                        .is_none()
                {
                    self.vaults.apk.signing = true;
                    self.vaults.apk.review = None;
                    self.vaults.apk.which = None;
                }
            }
            V::ApkPick(k) => {
                if let Some((_, r)) = self.apk_selected()
                    && let Some(fp) = crate::apk::fingerprint_of(&r)
                    && self
                        .backup_waits(crate::backup_flow::Kind::Apk, &fp)
                        .is_none()
                    && let Some(name) = self.inbox.get(k).map(|i| i.name.clone())
                {
                    self.apk_open_review(&name, fp);
                }
            }
            V::ApkHoldSign => self.apk_sign(),
            V::ApkSecond => self.vaults.focus = Some(Focus::Second),
            V::ApkBack => {
                if self.vaults.apk.review.is_some() {
                    self.vaults.apk.review = None;
                    self.vaults.apk.second.clear();
                    self.vaults.focus = None;
                } else {
                    self.vaults.apk.leave();
                }
            }
            V::ApkFromFiles(k) => self.apk_from_files(k),
            V::ApkKeyPick(v, at) => self.apk_key_picked(v, at),
        }
    }

    /// Whether category `k` is offered: the APK signing keys only with APK
    /// signing on or an open vault that holds one (F59).
    pub(crate) fn category_listed(&self, k: usize) -> bool {
        k != APK_CATEGORY
            || self.apk_signing
            || self.vaults.open.iter().any(|v| {
                v.contents
                    .records
                    .iter()
                    .any(|r| CATEGORIES[APK_CATEGORY].1.contains(&r.kind))
            })
    }

    fn vault_import_entries(&mut self, files: &[usize]) {
        if self.vaults.open.is_empty() {
            self.toast(EN.vaults_unlock_a_vault);
            return;
        }
        let mut records = Vec::new();
        for &k in files {
            if let Some(it) = self.inbox.get(k) {
                records.extend(fv::entries::parse(&String::from_utf8_lossy(&it.bytes)));
            }
        }
        let n = records.len();
        let Some(v) = self.vaults.open.get_mut(self.vaults.current) else {
            return;
        };
        let mut trial = v.contents.clone();
        trial.records.extend(records.iter().cloned());
        if trial.used() > v.header().slot_len as usize {
            self.toast(EN.vaults_those_entries);
            return;
        }
        v.contents.records.extend(records);
        if n > 0 {
            v.changes += 1;
        }
        self.vaults.category = 2;
        self.vaults.form = None;
        self.vaults.saving = false;
        self.screen = Screen::VaultContents;
        self.toast(&fill(
            EN.vaults_added,
            &[
                &n.to_string(),
                (if n == 1 {
                    EN.vaults_entry_inline
                } else {
                    EN.vaults_entries_inline
                }),
            ],
        ));
    }

    /// A TOTP setup code the camera read, into a new entry's form; text of
    /// several entries, into the vault.
    pub(crate) fn vault_scanned_entry(&mut self, text: &str) -> Result<(), String> {
        let t = text.trim();
        // Scanned from the codes screen (`docs/TOTP.md` §5.3): back to
        // it, with the new row in the list. Scanned from the Add… menu
        // or the entries list's own button: back to Vault contents, as
        // before.
        let back = if self.screen == Screen::Codes {
            Screen::Codes
        } else {
            Screen::VaultContents
        };
        if let Some((title, account)) = fv::entries::otpauth_names(t) {
            self.vaults.category = 2;
            self.vault_form(false);
            if let Some(f) = self.vaults.form.as_mut() {
                for (n, _, b, _) in f.fields.iter_mut() {
                    match *n {
                        field::TITLE => b.set(&title),
                        field::USERNAME => b.set(&account),
                        field::TOTP => b.set(t),
                        _ => {}
                    }
                }
            }
            self.screen = back;
            return Ok(());
        }
        if fv::entries::looks_like_entries(t) {
            let records = fv::entries::parse(t);
            if let Some(v) = self.vaults.open.get_mut(self.vaults.current) {
                let n = records.len();
                v.contents.records.extend(records);
                v.changes += 1;
                self.vaults.category = 2;
                self.screen = back;
                self.toast(&fill(
                    EN.vaults_added,
                    &[
                        &n.to_string(),
                        (if n == 1 {
                            EN.vaults_entry_inline
                        } else {
                            EN.vaults_entries_inline
                        }),
                    ],
                ));
                return Ok(());
            }
        }
        Err(EN.vaults_not_a_totp_setup.to_string())
    }

    fn vault_prompt_go(&mut self) {
        let Some(p) = self.vaults.prompt.as_mut() else {
            return;
        };
        match p.purpose {
            Purpose::ForWallet(v, r) => self.pass_seed_try(v, r),
            Purpose::Backup(_) => {
                if p.text.text.is_empty() {
                    p.error = Some(EN.vaults_type_the_backup.to_string());
                } else {
                    self.vaults.working = Some(Work::Backup);
                    self.vaults.drawn = false;
                }
            }
            Purpose::Kdbx => {
                if p.text.text.chars().count() < 8 {
                    p.error = Some(EN.vaults_a_passphrase.to_string());
                } else {
                    self.vaults.working = Some(Work::Kdbx);
                    self.vaults.drawn = false;
                }
            }
            Purpose::KeyPassphrase => {
                let passphrase = Zeroizing::new(p.text.text.to_string());
                let Some(at) = self.vault_selected_index() else {
                    return;
                };
                let Some(r) = self
                    .vaults
                    .open
                    .get(self.vaults.current)
                    .and_then(|v| v.contents.records.get(at))
                else {
                    return;
                };
                let Some(m) = crate::wallet::record_words(r) else {
                    return;
                };
                let words = phrase_of(&m);
                let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
                match self
                    .session
                    .add_words_with(&words, &passphrase, &label, None)
                {
                    Ok(fp) => {
                        self.vaults.prompt = None;
                        self.vaults.focus = None;
                        self.refresh_spend();
                        self.toast(&fill(
                            EN.vaults_key_is_loaded,
                            &[&crate::wallet::fp_text(fp).to_string()],
                        ));
                    }
                    Err(e) => {
                        if let Some(p) = self.vaults.prompt.as_mut() {
                            p.error = Some(e.text());
                        }
                    }
                }
            }
        }
    }

    /// Opens the backup the prompt names and keeps what it holds in the
    /// open vault.
    fn vault_open_backup(&mut self) {
        let Some(p) = self.vaults.prompt.as_ref() else {
            return;
        };
        let Purpose::Backup(k) = p.purpose else {
            return;
        };
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let label = crate::stem(&item.name);
        let opened = osk_backup::oskb::open_payload(&item.bytes, p.text.text.as_bytes());
        use osk_backup::oskb::Opened;
        let (record, category) = match opened {
            Ok(Opened::Words(m)) => (
                Record::new(kind::KEY)
                    .with(field::KEY, &records::words_payload(&m))
                    .with(field::KEY_LABEL, label.as_bytes()),
                0,
            ),
            Ok(Opened::Seed(seed)) => {
                let mut payload = Zeroizing::new(vec![0u8; 1 + osk_backup::oskb::MAX_SEED]);
                payload[0] = seed.len() as u8;
                payload[1..1 + seed.len()].copy_from_slice(&seed);
                (
                    Record::new(kind::KEY)
                        .with(field::KEY, &payload)
                        .with(field::KEY_LABEL, label.as_bytes()),
                    0,
                )
            }
            Ok(Opened::Note(text)) => (Record::new(kind::NOTE).with(field::NOTE, &text), 3),
            Ok(Opened::Sheet(sheet)) => {
                match osk_backup::oskb::sheet_payload(&sheet.descriptor, &sheet.name, &sheet.note) {
                    Some(payload) => (Record::new(kind::SHEET).with(field::SHEET, &payload), 3),
                    None => return,
                }
            }
            Err(e) => {
                let why = match e {
                    osk_backup::oskb::Error::Passphrase => EN.vaults_that_passphrase.to_string(),
                    osk_backup::oskb::Error::Memory(mib) => {
                        fill(EN.vaults_needs_mib_which, &[&mib.to_string()])
                    }
                    osk_backup::oskb::Error::Format => EN.vaults_not_a_backup.to_string(),
                };
                if let Some(p) = self.vaults.prompt.as_mut() {
                    p.text.clear();
                    p.error = Some(why);
                }
                return;
            }
        };
        self.vaults.prompt = None;
        self.vaults.focus = None;
        self.vaults.category = category;
        self.vault_push(
            record,
            &fill(EN.vaults_is_in_the_vault, &[&label.to_string()]),
        );
    }

    /// Asks for the one-pass measurement, once.
    pub(crate) fn vault_measure(&mut self) {
        if self.vaults.ms_per_unit.is_none() && self.vaults.working.is_none() {
            self.vaults.working = Some(Work::Measure);
            self.vaults.drawn = false;
        }
    }

    /// What a tick does for vaults: runs work whose frame has been drawn,
    /// finishes a measurement, and fires a hold. Returns whether to redraw.
    pub(crate) fn vault_tick(&mut self, now_ms: u64) -> bool {
        let mut redraw = false;
        if let Some(t0) = self.vaults.measure_from.take() {
            self.vaults.ms_per_unit = Some(now_ms.saturating_sub(t0).max(1));
            redraw = true;
        }
        if let Some(work) = self.vaults.working
            && self.vaults.drawn
        {
            self.vaults.working = None;
            match work {
                Work::Measure => {
                    let header = Header {
                        cost: Cost {
                            memory_kib: 64 * 1024,
                            passes: 1,
                            lanes: 1,
                        },
                        slot_len: fv::SLOT_SIZES[0],
                        salt: [0; 32],
                    };
                    self.vaults.measure_from = Some(now_ms);
                    let _ = fv::derive(&header, b"measure");
                }
                Work::Unlock => self.vault_unlock(),
                Work::Backup => self.vault_open_backup(),
                Work::FlowMake => self.flow_make_work(),
                Work::Kdbx => self.vault_kdbx(),
                Work::Create => self.vault_create(),
            }
            redraw = true;
        }
        if let Some((Action::Vault(held), _)) = self.pressed
            && is_hold(Action::Vault(held))
        {
            if now_ms.saturating_sub(self.vaults.pressed_at) >= HOLD_MS {
                self.pressed = None;
                self.vault_act(held);
            }
            redraw = true;
        }
        if let Some((Action::Flow(held), _)) = self.pressed
            && is_hold(Action::Flow(held))
        {
            if now_ms.saturating_sub(self.vaults.pressed_at) >= HOLD_MS {
                self.pressed = None;
                self.flow_act(held);
            }
            redraw = true;
        }
        redraw
    }

    fn vault_unlock_check(&mut self) {
        let files = self.vault_files();
        let Some(f) = files.get(self.vaults.pick) else {
            return;
        };
        let error = if !self.may_load_keys() {
            Some(fill(EN.vaults_remove_the_to, &[self.medium.noun()]))
        } else if self.vaults.passphrase.text.is_empty() {
            Some(EN.vaults_type_the_passphrase.to_string())
        } else {
            let need = vault_need_mib(f.header.cost.memory_kib / 1024, f.header.slot_len);
            match self.vaults.memory_free_mib {
                Some(free) if need > free => Some(fill(
                    EN.vaults_needs_this_computer,
                    &[&mib_text(need).to_string(), &mib_text(free).to_string()],
                )),
                _ => None,
            }
        };
        self.vaults.unlock_error = error;
        if self.vaults.unlock_error.is_none() {
            self.vaults.working = Some(Work::Unlock);
            self.vaults.drawn = false;
        }
    }

    fn vault_unlock(&mut self) {
        let before = self.vaults.open.len();
        self.vault_unlock_inner();
        // Unlocked inside a backup flow: it goes on, and loads what it
        // needs itself, so no Load sheet waits over it (F62).
        if self.vaults.open.len() > before {
            let screen = self.screen;
            self.flow_vault_opened();
            let from_tab = self.vaults.load.as_ref().is_some_and(|l| !l.boot);
            if self.screen != screen && from_tab && self.sheet == Some(crate::Sheet::UnlockLoad) {
                self.vaults.load = None;
                self.sheet = None;
            }
        }
    }

    fn vault_unlock_inner(&mut self) {
        let files = self.vault_files();
        let Some(f) = files.get(self.vaults.pick) else {
            return;
        };
        let Some(bytes) = self.vault_bytes(f) else {
            return;
        };
        let name = f.name.clone();
        // Unlocked on the boot import's sheet: the unlock imports.
        let for_import = self.sheet == Some(crate::Sheet::Import) && self.import.is_some();
        let result = fv::open(&bytes, self.vaults.passphrase.text.as_bytes());
        match result {
            Ok((opened, contents)) => {
                self.vaults.passphrase.clear();
                self.vaults.typed_shown = false;
                self.vaults.focus = None;
                self.vaults.open.push(OpenVault {
                    name,
                    file: bytes,
                    opened,
                    contents: VaultRecords::new(contents),
                    changes: 0,
                    skip: Default::default(),
                    each_shown: false,
                });
                self.vaults.current = self.vaults.open.len() - 1;
                // Unlocked for a vault category from Tools: that category.
                // Otherwise the first kind it holds.
                if self.vaults.back_to != Some(Screen::VaultContents) {
                    let held = |kinds: &[u8]| {
                        self.vaults.open[self.vaults.open.len() - 1]
                            .contents
                            .records
                            .iter()
                            .any(|r| kinds.contains(&r.kind))
                    };
                    // Its wallets first, each with its chart (§7.2).
                    self.vaults.category = if held(CATEGORIES[1].1) {
                        1
                    } else {
                        CATEGORIES
                            .iter()
                            .position(|(_, kinds)| held(kinds))
                            .unwrap_or(0)
                    };
                    self.vaults.category_chosen = false;
                } else {
                    self.vaults.category_chosen = true;
                }
                self.vaults.item = [0; 7];
                // Keys and wallets set not to load at unlock start
                // unchosen on the Vaults tab's load list.
                let v = self.vaults.current;
                let skip = self.vaults.open[v]
                    .contents
                    .records
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| {
                        matches!(r.kind, kind::KEY | kind::WALLET) && !records::loads_at_unlock(r)
                    })
                    .map(|(i, _)| i)
                    .collect();
                self.vaults.open[v].skip = skip;
                let records = self.vaults.open[v].contents.records.clone();
                self.amounts_from_vault(&records);
                // The vault just made opens on its contents, empty, with
                // what to do next.
                let made = self.vaults.just_made.take() == Some(self.vaults.open[v].name.clone());
                self.toast(&fill(
                    EN.vaults_unlocked,
                    &[&self.vaults.open[v].name.to_string()],
                ));
                self.vault_summaries_refresh();
                if for_import {
                    // Its Load sheet (F62); after it, the next vault the
                    // import brought is offered, or, with none left, the
                    // landing (F60).
                    self.vaults.back_to = None;
                    let before = self
                        .import
                        .as_ref()
                        .map_or(self.session.wallets.len(), |i| i.wallets_before);
                    let label = self.import.as_ref().map(|i| i.label.clone());
                    if self.import_unlocked(v) {
                        let file = self.vaults.open[v].name.clone();
                        self.unlock_landed(before, &file, label.as_deref());
                    }
                    return;
                }
                // Every vault the boot import brought unlocked elsewhere:
                // the import is over (F60).
                self.import_settle();
                // Unlocked from the Spend tab, or for it: everything loads.
                let back = self.vaults.back_to.take();
                if self.screen == Screen::Family || back == Some(Screen::Family) {
                    self.screen = Screen::Family;
                    self.family_unlocked(v);
                } else if back.is_none() && !made {
                    // The Vaults list's Unlock (`docs/NEW-WALLET.md`
                    // §11.1a, F62): Files, where it landed before F62
                    // when it loaded no wallet, with the vault's Load
                    // sheet over it; a wallet loaded lands on Wallets as
                    // the boot sheet's does. A vault opened from inside a flow, or
                    // just made, keeps `back_to` or `made` and takes the
                    // branch below instead, loading nothing here.
                    self.screen = Screen::Files;
                    let before = self.session.wallets.len();
                    self.load_sheet_offer(v, false, before);
                } else {
                    self.screen = back.unwrap_or(if made {
                        Screen::VaultContents
                    } else {
                        Screen::Files
                    });
                }
                // Landed on the codes screen with the clock not checked
                // this power-on: Set the time first (`docs/TOTP.md`
                // §5.2), coming back to the codes.
                if self.screen == Screen::Codes && !self.clock_checked() {
                    self.open_set_time();
                }
                // Unlocked for the boot import: its sheet again, with the
                // vault's wallets and keys in it.
                if back == Some(Screen::Home) && self.import.is_some() {
                    self.sheet = Some(crate::Sheet::Import);
                }
                // Unlocked for a secret: its sheet again, with the vault.
                if back.is_some() && self.secret_out.is_some() {
                    self.sheet = Some(crate::Sheet::SecretOut);
                }
            }
            Err(e) => {
                self.vaults.passphrase.clear();
                self.vaults.unlock_error = Some(e.reason());
            }
        }
    }

    /// After an unlock's Load sheet that loaded wallets
    /// (`docs/NEW-WALLET.md` §11.1, §11.1a, F62: the boot sheet's Unlock or
    /// the Vaults list's): Wallets, on
    /// the first loaded that can sign here, else the first loaded, and a
    /// toast naming what loaded from where. One that loaded none stays
    /// where it landed.
    pub(crate) fn unlock_landed(&mut self, before: usize, file: &str, stick: Option<&str>) {
        let loaded: Vec<usize> = (before..self.session.wallets.len()).collect();
        let Some(&first) = loaded.first() else {
            return;
        };
        if self.sheet.is_some() {
            return;
        }
        let signs = loaded.iter().copied().find(|&i| {
            let w = &self.session.wallets[i];
            self.session.slots(w).iter().any(|s| s.held_by.is_some())
        });
        self.wallet = signs.unwrap_or(first);
        self.screen = Screen::Wallets;
        self.card_offset = 0.0;
        self.glance = false;
        // From the vault, or from the stick it came on when the stick's
        // own files brought some.
        let from = match stick {
            Some(s)
                if loaded
                    .iter()
                    .any(|&i| self.session.wallets[i].source != EN.vaults_vault) =>
            {
                s
            }
            _ => file,
        };
        let what = match loaded.len() {
            1 => self.session.wallets[first].name.clone(),
            n => fill(EN.vaults_wallets_n, &[&n.to_string()]),
        };
        self.toast(&fill(EN.vaults_loaded_from, &[&what.to_string(), from]));
    }

    /// The keys and wallets open vault `v` holds, each with whether the
    /// session has it already and whether it is chosen for the next load.
    /// Kept in the drawing memo while a frame is drawn (`docs/SPEED.md`
    /// §3.3). Reads its wallet records as it keeps them read
    /// ([`VaultRecords`]).
    pub fn vault_rows(&self, v: usize) -> Vec<VaultRow> {
        if !self.drawing.is_on() {
            return self.vault_rows_now(v);
        }
        if let Some((_, kept)) = self
            .drawing
            .vault_rows
            .borrow()
            .iter()
            .find(|(o, _)| *o == v)
        {
            return kept.clone();
        }
        let rows = crate::work::filling(|| self.vault_rows_now(v));
        self.drawing.vault_rows.borrow_mut().push((v, rows.clone()));
        rows
    }

    /// Works out [`Faraday::vault_rows`] from the wallet records as the
    /// vault keeps them read ([`VaultRecords`]).
    fn vault_rows_now(&self, v: usize) -> Vec<VaultRow> {
        let Some(open) = self.vaults.open.get(v) else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        for (i, r) in open.contents.of(kind::WALLET) {
            let described = open.contents.described(i);
            let loaded = described.is_some_and(|d| {
                self.session
                    .wallets
                    .iter()
                    .any(|w| crate::wallet::same_wallet(w.policy()) == d.same)
            });
            rows.push(VaultRow {
                record: i,
                name: r
                    .text(field::WALLET_NAME)
                    .unwrap_or(EN.vaults_wallet)
                    .to_string(),
                detail: described.map(|d| d.shape.clone()).unwrap_or_default(),
                seed: false,
                loaded,
                chosen: !open.skip.contains(&i),
            });
        }
        for (i, r) in open.contents.of(kind::KEY) {
            let fp = crate::vault_screens::key_fingerprint(self, r).unwrap_or_default();
            let loaded = self
                .session
                .keys
                .iter()
                .any(|k| crate::wallet::fp_text(k.fingerprint()) == fp);
            rows.push(VaultRow {
                record: i,
                name: r
                    .text(field::KEY_LABEL)
                    .unwrap_or(EN.vaults_seed)
                    .to_string(),
                detail: fp,
                seed: true,
                loaded,
                chosen: !open.skip.contains(&i),
            });
        }
        rows
    }

    /// Where the seed with master fingerprint `fp` is in the open vaults:
    /// the vault and its record.
    pub fn vault_key_for(&self, fp: osk_bip::keys::Fingerprint) -> Option<(usize, usize)> {
        let want = crate::wallet::fp_text(fp);
        self.vaults.open.iter().enumerate().find_map(|(v, open)| {
            open.contents
                .of(kind::KEY)
                .find(|(_, r)| {
                    crate::vault_screens::key_fingerprint(self, r).as_deref() == Some(&want)
                })
                .map(|(i, _)| (v, i))
        })
    }

    /// The wallets open vault `v` holds together with some of their
    /// seeds, each with those seeds' records.
    pub fn vault_wallets_with_keys(&self, v: usize) -> Vec<VaultWallet> {
        let Some(open) = self.vaults.open.get(v) else {
            return Vec::new();
        };
        let seeds: Vec<(usize, String)> = open
            .contents
            .of(kind::KEY)
            .filter_map(|(i, r)| crate::vault_screens::key_fingerprint(self, r).map(|f| (i, f)))
            .collect();
        let rows = self.vault_rows(v);
        let loaded = |i: usize| rows.iter().any(|r| r.record == i && r.loaded);
        let mut out = Vec::new();
        for (i, r, d) in open.contents.described_of(kind::WALLET) {
            let policy = d.policy.clone();
            let name = r
                .text(field::WALLET_NAME)
                .unwrap_or(EN.vaults_wallet)
                .to_string();
            let wallet = crate::wallet::Wallet::new(
                name.clone(),
                policy,
                String::new(),
                crate::wallet::Origin::Named,
            );
            let fps: Vec<String> = wallet
                .policy()
                .keys()
                .iter()
                .filter_map(|k| k.fingerprint())
                .map(crate::wallet::fp_text)
                .collect();
            let keys: Vec<usize> = seeds
                .iter()
                .filter(|(_, f)| fps.contains(f))
                .map(|(k, _)| *k)
                .collect();
            if keys.is_empty() {
                continue;
            }
            out.push(VaultWallet {
                record: i,
                name,
                needed: crate::wallet::needed(&wallet),
                total: fps.len().max(1),
                done: loaded(i) && keys.iter().all(|&k| loaded(k)),
                chosen: !open.skip.contains(&i),
                keys,
            });
        }
        out
    }

    /// Loads the keys and wallets of an open vault into the session:
    /// every one, or the chosen ones.
    pub(crate) fn vault_load(&mut self, v: usize, all: bool) -> (usize, usize) {
        let Some(open) = self.vaults.open.get(v) else {
            return (0, 0);
        };
        let set: BTreeSet<usize> = (0..open.contents.records.len())
            .filter(|i| all || !open.skip.contains(i))
            .collect();
        self.vault_load_set(v, &set)
    }

    /// Loads wallet record `r` of open vault `v`, with the seeds of it the
    /// vault keeps where keys may load. Returns the wallet's place in the
    /// session.
    pub(crate) fn vault_load_wallet(&mut self, v: usize, r: usize) -> Option<usize> {
        let open = self.vaults.open.get(v)?;
        open.contents
            .records
            .get(r)
            .filter(|x| x.kind == kind::WALLET)?;
        let described = open.contents.described(r)?;
        let policy = described.policy.clone();
        let want = described.same.clone();
        let mut set = BTreeSet::from([r]);
        if self.may_load_keys() {
            let fps: Vec<String> = policy
                .keys()
                .iter()
                .filter_map(|k| k.fingerprint())
                .map(crate::wallet::fp_text)
                .collect();
            set.extend(open.contents.of(kind::KEY).filter_map(|(i, k)| {
                crate::vault_screens::key_fingerprint(self, k)
                    .filter(|f| fps.contains(f))
                    .map(|_| i)
            }));
        }
        let (keys, _) = self.vault_load_set(v, &set);
        if keys > 0 {
            self.toast(&fill(
                EN.vaults_loaded,
                &[
                    &keys.to_string(),
                    (if keys == 1 {
                        EN.vaults_seed_inline
                    } else {
                        EN.vaults_seeds
                    }),
                ],
            ));
        }
        self.session
            .wallets
            .iter()
            .position(|w| crate::wallet::same_wallet(w.policy()) == want)
    }

    /// Loads the records `set` names of open vault `v`: its keys and
    /// wallets among them.
    pub(crate) fn vault_load_set(&mut self, v: usize, set: &BTreeSet<usize>) -> (usize, usize) {
        let mut keys = 0;
        let mut wallets = 0;
        let Some(open) = self.vaults.open.get(v) else {
            return (0, 0);
        };
        let mut words: Vec<(Zeroizing<String>, String)> = Vec::new();
        let mut descs: Vec<(osk_bip::policy::WalletPolicy, String)> = Vec::new();
        let mut seeds: Vec<(Zeroizing<Vec<u8>>, String)> = Vec::new();
        let mut passphrases: Vec<Zeroizing<String>> = Vec::new();
        for (i, r) in open.contents.of(kind::KEY) {
            if !set.contains(&i) {
                continue;
            }
            let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
            if let Some(m) = crate::wallet::record_words(r) {
                // Stored with the key, or kept alone in an open vault.
                let pass = match r.text(field::KEY_PASSPHRASE) {
                    Some(p) => Zeroizing::new(p.to_string()),
                    None => self.passphrase_kept_for(&m).unwrap_or_default(),
                };
                words.push((phrase_of(&m), label));
                passphrases.push(pass);
            } else if let Some(seed) = seed_of(r) {
                seeds.push((seed, label));
            }
        }
        for (i, r) in open.contents.of(kind::WALLET) {
            if !set.contains(&i) {
                continue;
            }
            // Read once when the records changed (`VaultRecords`), not
            // again here.
            if let Some(d) = open.contents.described(i) {
                descs.push((
                    d.policy.clone(),
                    r.text(field::WALLET_NAME)
                        .unwrap_or(EN.vaults_wallet)
                        .to_string(),
                ));
            }
        }
        for ((w, label), p) in words.iter().zip(&passphrases) {
            if self.session.add_words_with(w, p, label, None).is_ok() {
                keys += 1;
            }
        }
        for (seed, label) in &seeds {
            if self.session.add_seed(seed, label).is_ok() {
                keys += 1;
            }
        }
        let before = self.session.wallets.len();
        for (policy, name) in descs {
            let _ = self.session.add_policy(
                &name,
                policy,
                EN.vaults_vault,
                crate::wallet::Origin::Vault,
            );
        }
        wallets += self.session.wallets.len() - before;
        self.refresh_spend();
        (keys, wallets)
    }

    /// The selected entry as a KeePass database sealed under the prompt's
    /// passphrase, to the Outbox: sealed, so it goes as it is.
    fn vault_kdbx(&mut self) {
        use osk_backup::kdbx;
        let Some(at) = self.vault_selected_index() else {
            return;
        };
        let Some(r) = self
            .vaults
            .open
            .get(self.vaults.current)
            .and_then(|v| v.contents.records.get(at))
            .filter(|r| r.kind == kind::ENTRY)
            .cloned()
        else {
            return;
        };
        let Some(passphrase) = self
            .vaults
            .prompt
            .as_ref()
            .map(|p| Zeroizing::new(p.text.text.to_string()))
        else {
            return;
        };
        // The database's own randomness: one fresh draw, spread over the
        // bytes the format needs.
        let Some(draw) = self.fresh(b"kdbx") else {
            if let Some(p) = self.vaults.prompt.as_mut() {
                p.error = Some(EN.vaults_no_randomness.to_string());
            }
            return;
        };
        let mut seed = Zeroizing::new([0u8; kdbx::SEED_LEN]);
        for (i, chunk) in seed.chunks_mut(32).enumerate() {
            let mut e = sha256::Hash::engine();
            e.input(&draw);
            e.input(&(i as u32).to_le_bytes());
            let h = sha256::Hash::from_engine(e).to_byte_array();
            chunk.copy_from_slice(&h[..chunk.len()]);
        }
        let title = r.text(field::TITLE).unwrap_or(EN.vaults_entry).to_string();
        let mut notes = String::new();
        for (label, f) in [
            (EN.vaults_username, field::USERNAME),
            (EN.vaults_url, field::URL),
        ] {
            if let Some(v) = r.text(f) {
                notes.push_str(&format!("{label}: {v}\n"));
            }
        }
        if let Some(n) = r.text(field::NOTES) {
            notes.push_str(n);
        }
        let notes = Zeroizing::new(notes);
        let item = kdbx::Item {
            title: title.as_bytes(),
            notes: notes.as_bytes(),
            secret: r.field(field::PASSWORD).unwrap_or(&[]),
        };
        match kdbx::write(
            &[item],
            passphrase.as_bytes(),
            osk_backup::DEVICE_PARAMS,
            &seed,
        ) {
            Some(file) => {
                let stem: String = crate::file_stem(&title);
                let name = format!("{stem}.kdbx");
                self.vaults.prompt = None;
                self.vaults.focus = None;
                self.put_outbox(&name, file);
                self.toast(&fill(
                    EN.vaults_waits_sealed,
                    &[&name.to_string(), self.medium.for_the()],
                ));
            }
            None => {
                if let Some(p) = self.vaults.prompt.as_mut() {
                    p.error = Some(EN.vaults_that_entry_is.to_string());
                }
            }
        }
    }

    /// The position in the slot of the item selected in the kind shown.
    pub fn vault_selected_index(&self) -> Option<usize> {
        let v = self.vaults.open.get(self.vaults.current)?;
        let kinds = CATEGORIES[self.vaults.category].1;
        let items: Vec<usize> = v
            .contents
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| kinds.contains(&r.kind))
            .map(|(i, _)| i)
            .collect();
        let k = self.vaults.item[self.vaults.category].min(items.len().saturating_sub(1));
        items.get(k).copied()
    }

    fn vault_delete(&mut self) {
        let Some(at) = self.vault_selected_index() else {
            return;
        };
        if let Some(v) = self.vaults.open.get_mut(self.vaults.current) {
            v.contents.records.remove(at);
            v.changes += 1;
            self.toast(EN.vaults_deleted_from);
        }
    }

    fn vault_load_selected(&mut self) {
        let Some(at) = self.vault_selected_index() else {
            return;
        };
        let Some(r) = self
            .vaults
            .open
            .get(self.vaults.current)
            .and_then(|v| v.contents.records.get(at))
        else {
            return;
        };
        match r.kind {
            kind::KEY => {
                let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
                let added = match crate::wallet::record_words(r) {
                    Some(m) => {
                        // Stored with the key, or kept alone in an open
                        // vault (`docs/BACKUP-FLOW.md` §19.12 item 4).
                        let p = match r.text(field::KEY_PASSPHRASE) {
                            Some(p) => Zeroizing::new(p.to_string()),
                            None => self.passphrase_kept_for(&m).unwrap_or_default(),
                        };
                        self.session.add_mnemonic(&m, &p, &label, None)
                    }
                    None => match seed_of(r) {
                        Some(seed) => self.session.add_seed(&seed, &label),
                        None => return,
                    },
                };
                match added {
                    Ok(fp) => self.toast(&fill(
                        EN.vaults_key_is_loaded,
                        &[&crate::wallet::fp_text(fp).to_string()],
                    )),
                    Err(e) => self.toast(&e.text()),
                }
            }
            kind::WALLET => {
                let text = r.text(field::WALLET).unwrap_or("").to_string();
                let name = r
                    .text(field::WALLET_NAME)
                    .unwrap_or(EN.vaults_wallet)
                    .to_string();
                match self.session.add_wallet(
                    &name,
                    &text,
                    EN.vaults_vault,
                    crate::wallet::Origin::Vault,
                ) {
                    Ok(i) => {
                        self.wallet = i;
                        self.screen = Screen::Wallets;
                    }
                    Err(e) => self.toast(&e.text()),
                }
            }
            _ => {}
        }
        self.refresh_spend();
    }

    /// The form that asks why the selected GPG key is revoked: its
    /// optional description, with the reason chosen by buttons. Its one
    /// field is number 3, which the Make form does not have.
    pub(crate) fn vault_form_for_revocation(&mut self) {
        self.vaults.signing = false;
        self.vaults.revoke_reason = 0;
        self.vaults.form = Some(Form {
            kind: kind::GPG,
            record: self.vault_selected_index(),
            fields: vec![(3, EN.vaults_description, TextBox::default(), false)],
        });
        self.vaults.saving = false;
        self.vaults.focus = Some(Focus::Field(0));
    }

    fn vault_form(&mut self, edit: bool) {
        let kinds = CATEGORIES[self.vaults.category].1;
        let at = if edit {
            self.vault_selected_index()
        } else {
            None
        };
        let existing = at.and_then(|i| {
            self.vaults
                .open
                .get(self.vaults.current)?
                .contents
                .records
                .get(i)
        });
        let k = existing.map(|r| r.kind).unwrap_or(kinds[0]);
        let shape: &[(u8, &'static str, bool)] = match k {
            kind::ENTRY => &[
                (field::TITLE, EN.vaults_title, false),
                (field::USERNAME, EN.vaults_username, false),
                (field::PASSWORD, EN.vaults_password, true),
                (field::URL, EN.vaults_url, false),
                (field::NOTES, EN.vaults_notes, false),
                (field::TOTP, EN.vaults_totp_secret, true),
            ],
            kind::NOTE => &[(field::NOTE, EN.vaults_text, false)],
            _ => return,
        };
        let fields = shape
            .iter()
            .map(|(n, label, secret)| {
                let mut b = TextBox::default();
                if let Some(t) = existing.and_then(|r| r.text(*n)) {
                    b.set(t);
                }
                (*n, *label, b, *secret)
            })
            .collect();
        self.vaults.form = Some(Form {
            kind: k,
            record: at,
            fields,
        });
        self.vaults.saving = false;
        self.vaults.focus = Some(Focus::Field(0));
    }

    fn vault_form_save(&mut self) {
        // The backup flow's Make form is saved by Make the key.
        if self.screen == Screen::BackupFlow {
            return self.flow_act(crate::backup_flow_steps::FlowAction::Make);
        }
        let Some(form) = self.vaults.form.take() else {
            return;
        };
        self.vaults.focus = None;
        if form.kind == kind::GPG && self.time_asked() {
            self.vaults.form = Some(form);
            return;
        }
        if form.kind == kind::GPG && form.fields.first().is_some_and(|f| f.0 == 3) {
            let reason = faraday_pgp::Reason::from_menu(self.vaults.revoke_reason)
                .unwrap_or(faraday_pgp::Reason::Unspecified);
            let text = form.fields[0].2.text.to_string();
            self.gpg_make_revocation(reason, &text);
            return;
        }
        let Some(v) = self.vaults.open.get_mut(self.vaults.current) else {
            return;
        };
        if form.kind == kind::SLOT_LABEL {
            let name = form.fields[0].2.text.trim().to_string();
            v.contents.records.retain(|r| r.kind != kind::SLOT_LABEL);
            if !name.is_empty() {
                let name: String = name.chars().take(64).collect();
                let mut name = name;
                while name.len() > 64 {
                    name.pop();
                }
                v.contents.records.insert(
                    0,
                    Record::new(kind::SLOT_LABEL).with(field::LABEL, name.as_bytes()),
                );
            }
            v.changes += 1;
            return;
        }
        if form.fields[0].2.text.trim().is_empty() {
            let what = if form.kind == kind::NOTE {
                EN.vaults_the_note_is
            } else {
                EN.vaults_an_entry_needs
            };
            self.vaults.form = Some(form);
            self.toast(what);
            return;
        }
        let mut record = match form.record.and_then(|i| v.contents.records.get(i)) {
            Some(r) => r.clone(),
            None => Record::new(form.kind),
        };
        // The TOTP secret field, if any, goes through `normalise` with
        // the form's own Title and Username (`docs/TOTP.md` §6); checked
        // before anything moves `form` away.
        let title = form.fields[0].2.text.to_string();
        let account = form
            .fields
            .iter()
            .find(|(n, ..)| *n == field::USERNAME)
            .map(|(_, _, b, _)| b.text.to_string())
            .unwrap_or_default();
        let totp_text = form
            .fields
            .iter()
            .find(|(n, ..)| *n == field::TOTP)
            .map(|(_, _, b, _)| b.text.to_string());
        let totp_store: Result<Option<String>, ()> = match &totp_text {
            None => Ok(None),
            Some(t) if t.trim().is_empty() => Ok(None),
            Some(t) => fv::totp::normalise(t, &title, &account).map(Some).ok_or(()),
        };
        let Ok(totp_store) = totp_store else {
            self.vaults.form = Some(form);
            self.toast(EN.vaults_not_a_totp_key);
            return;
        };
        for (n, _, b, _) in &form.fields {
            if *n != field::TOTP {
                record.set(*n, b.text.as_bytes());
            }
        }
        if totp_text.is_some() {
            match totp_store {
                Some(s) => record.set(field::TOTP, s.as_bytes()),
                None => record.set(field::TOTP, b""),
            }
        }
        // A new record must still fit in the slot.
        let mut trial = v.contents.clone();
        match form.record {
            Some(i) if i < trial.records.len() => trial.records[i] = record.clone(),
            _ => trial.records.push(record.clone()),
        }
        if trial.used() > v.header().slot_len as usize {
            self.vaults.form = Some(form);
            self.toast(EN.vaults_that_does_not);
            return;
        }
        let kinds = CATEGORIES[self.vaults.category].1;
        match form.record {
            Some(i) if i < v.contents.records.len() => v.contents.records[i] = record,
            _ => {
                v.contents.records.push(record);
                let n = v
                    .contents
                    .records
                    .iter()
                    .filter(|r| kinds.contains(&r.kind))
                    .count();
                self.vaults.item[self.vaults.category] = n.saturating_sub(1);
            }
        }
        v.changes += 1;
    }

    /// Inbox file k into the open vault, as the record its kind is.
    fn vault_add_file(&mut self, k: usize) {
        if self.vaults.open.is_empty() {
            self.toast(EN.vaults_unlock_a_vault);
            return;
        }
        let Some(it) = self.inbox.get(k) else {
            return;
        };
        let stem = crate::stem(&it.name);
        let text = Zeroizing::new(String::from_utf8_lossy(&it.bytes).into_owned());
        let (record, said, category) = match it.kind {
            FileKind::Text => {
                // A note's first line is its title.
                let note = Zeroizing::new(format!("{stem}\n{}", text.trim_end()));
                (
                    Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
                    fill(EN.vaults_is_in_the_vault_as_a_note, &[&it.name.to_string()]),
                    3,
                )
            }
            FileKind::Words => {
                let words = crate::forms::file_words(&text);
                let Ok(m) = crate::forms::typed_mnemonic(osk_bip::bip39::Language::English, &words)
                else {
                    return;
                };
                (
                    Record::new(kind::KEY)
                        .with(field::KEY, &records::words_payload(&m))
                        .with(field::KEY_LABEL, stem.as_bytes()),
                    fill(EN.vaults_is_in_the_vault_as_a, &[&it.name.to_string()]),
                    0,
                )
            }
            FileKind::Wallet => (
                Record::new(kind::WALLET)
                    .with(field::WALLET, text.trim().as_bytes())
                    .with(field::WALLET_NAME, stem.as_bytes()),
                fill(EN.vaults_is_in_the_vault_as, &[&it.name.to_string()]),
                1,
            ),
            _ => return,
        };
        let before = self.vaults.open.get(self.vaults.current).map(|v| v.changes);
        self.vault_push(record, &said);
        if self.vaults.open.get(self.vaults.current).map(|v| v.changes) != before {
            self.vaults.category = category;
        }
    }

    fn vault_save_key(&mut self, k: usize) {
        self.vault_save_key_with(k, false);
    }

    fn vault_save_key_with(&mut self, k: usize, with_passphrase: bool) {
        if let Some((record, label)) = self.key_record(k, with_passphrase) {
            self.vault_push(
                record,
                &fill(EN.vaults_is_in_the_vault, &[&label.to_string()]),
            );
        }
    }

    /// Costs a BIP-39 seed and a master key per passphrase record tried:
    /// the passphrase an open vault keeps alone (a type 14 record,
    /// `docs/BACKUP-FLOW.md` §19.12 item 4) for the key the words `m` make
    /// with it, found by the fingerprint the record names.
    pub(crate) fn passphrase_kept_for(
        &self,
        m: &osk_bip::bip39::Mnemonic,
    ) -> Option<Zeroizing<String>> {
        for open in &self.vaults.open {
            for (_, r) in open.contents.of(kind::PASSPHRASE) {
                let (Some(fp), Some(p)) =
                    (r.field(field::PASSPHRASE_KEY), r.text(field::PASSPHRASE))
                else {
                    continue;
                };
                let Ok(seed) = m.to_seed(p.as_bytes()) else {
                    continue;
                };
                crate::work::count(crate::work::Work::MasterKey);
                let master = osk_bip::keys::MasterKey::from_seed(&seed, self.session.network());
                if master.fingerprint().0[..] == *fp {
                    return Some(Zeroizing::new(p.to_string()));
                }
            }
        }
        None
    }

    /// Session key `k` as the vault's key record, ticked at unlock, and
    /// its label: what Save on Vaults writes, and the secret sheet for a
    /// seed. With its BIP-39 passphrase only when `with_passphrase`.
    pub(crate) fn key_record(
        &mut self,
        k: usize,
        with_passphrase: bool,
    ) -> Option<(Record, String)> {
        let key = self.session.keys.get(k)?;
        let Some(words) = key.words.as_ref() else {
            self.toast(EN.vaults_this_key_has);
            return None;
        };
        let m = crate::wallet::parse_words(key.language, words).ok()?;
        // Made here and its backup not done: the vault says so, beside
        // the seed.
        // It loads at unlock (no flag says otherwise, F62).
        let mut record = Record::new(kind::KEY)
            .with(field::KEY, &records::words_payload(&m))
            .with(field::KEY_LABEL, key.label.as_bytes());
        if self.key_held_stored(key.fingerprint()) {
            record.push(field::KEY_FLAGS, &[records::BACKUP_PENDING]);
        }
        // A BIP-39 passphrase is stored only when the person asks
        // (`PLAN.md` §6.3).
        if with_passphrase && let Some(p) = key.passphrase.as_ref() {
            record.push(field::KEY_PASSPHRASE, p.as_bytes());
        }
        Some((record, key.label.clone()))
    }

    fn vault_save_wallet(&mut self, k: usize) {
        let Some(w) = self.session.wallets.get(k) else {
            return;
        };
        // A threshold wallet and a silent payments wallet are their
        // records: neither has a descriptor that reads back as it.
        let text = match (w.policy().record(), w.policy().silent()) {
            (Some(r), _) => r.to_text(),
            (None, Some(s)) => s.to_text(),
            (None, None) => w.policy().to_descriptor_checksummed(),
        };
        let record = Record::new(kind::WALLET)
            .with(field::WALLET, text.as_bytes())
            .with(field::WALLET_NAME, w.name.as_bytes());
        let name = w.name.clone();
        self.vault_push(
            record,
            &fill(EN.vaults_is_in_the_vault, &[&name.to_string()]),
        );
    }

    pub(crate) fn vault_push(&mut self, record: Record, said: &str) {
        self.vault_push_in(self.vaults.current, record, said);
    }

    /// Adds `record` to open vault `v`, saying `said`. False when it does
    /// not fit in the vault's slot.
    pub(crate) fn vault_push_in(&mut self, v: usize, record: Record, said: &str) -> bool {
        let Some(open) = self.vaults.open.get_mut(v) else {
            return false;
        };
        let mut trial = open.contents.clone();
        trial.records.push(record.clone());
        if trial.used() > open.header().slot_len as usize {
            self.toast(EN.vaults_that_does_not);
            return false;
        }
        open.contents.records.push(record);
        open.changes += 1;
        self.toast(said);
        true
    }

    fn vault_create_check(&mut self) {
        let free = self.vaults.memory_free_mib;
        let cost = self.vaults.form_cost();
        let medium = self.medium;
        let Some(c) = self.vaults.create.as_mut() else {
            return;
        };
        let error = if !self.sticks.is_empty() {
            Some(fill(EN.vaults_remove_the_first, &[medium.noun()]))
        } else if let Some(n) = c
            .steps()
            .iter()
            .position(|&i| !c.done[i as usize] && i != vstep::PHRASES)
        {
            c.open = Some(c.steps()[n]);
            Some(fill(EN.vaults_finish_step, &[&(n + 1).to_string()]))
        } else if c.phrases.iter().any(|(a, _)| a.text.is_empty()) {
            c.open = Some(vstep::PHRASES);
            Some(EN.vaults_every_passphrase.to_string())
        } else if (0..c.phrases.len()).any(|i| !Vaults::phrase_ok(c, i)) {
            c.open = Some(vstep::PHRASES);
            Some(EN.vaults_each_passphrase.to_string())
        } else if (0..c.phrases.len())
            .any(|i| (i + 1..c.phrases.len()).any(|j| *c.phrases[i].0.text == *c.phrases[j].0.text))
        {
            c.open = Some(vstep::PHRASES);
            Some(EN.vaults_two_passphrases.to_string())
        } else if let (Some(free), Some((_, mem, _))) = (free, cost)
            && vault_need_mib(mem, c.slot) > free
        {
            Some(EN.vaults_choose_a_cost.to_string())
        } else {
            None
        };
        c.error = error;
        if c.error.is_none() {
            c.done[vstep::PHRASES as usize] = true;
            self.vaults.working = Some(Work::Create);
            self.vaults.drawn = false;
        }
    }

    fn vault_create(&mut self) {
        // Everything random in a vault comes from fresh system entropy of
        // its own; a vault is never made before it has arrived.
        if self.pool.is_empty() {
            self.commands
                .push_back(osk_shell_api::Command::RequestEntropy);
            if let Some(c) = self.vaults.create.as_mut() {
                c.error = Some(EN.vaults_no_randomness.to_string());
            }
            return;
        }
        let Some((_, mem, passes)) = self.vaults.form_cost() else {
            return;
        };
        let Some(seed) = self.fresh(b"vault create") else {
            return;
        };
        let Some(c) = self.vaults.create.as_ref() else {
            return;
        };
        let cost = Cost {
            memory_kib: mem * 1024,
            passes,
            lanes: 1,
        };
        let first_phrase = Zeroizing::new(
            c.phrases
                .first()
                .map(|(a, _)| a.text.to_string())
                .unwrap_or_default(),
        );
        let phrases: Vec<&[u8]> = c.phrases.iter().map(|(a, _)| a.text.as_bytes()).collect();
        // Every slot starts with the vault's name: a slot opened under
        // pressure says nothing of the others (`docs/VAULT.md` §9, F57).
        let label = match c.name.text.trim() {
            "" => EN.vaults_vault,
            typed => {
                // A slot's label holds 64 bytes (`docs/VAULT.md` §7).
                let end = (0..=typed.len().min(64))
                    .rev()
                    .find(|&i| typed.is_char_boundary(i))
                    .unwrap_or(0);
                &typed[..end]
            }
        };
        let contents: Vec<Contents> = phrases
            .iter()
            .map(|_| Contents {
                records: vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, label.as_bytes())],
            })
            .collect();
        let made = fv::create(c.slot, cost, &phrases, &contents, &seed);
        match made {
            Ok(file) => {
                let name = self.free_vault_name(&vault_stem(&c.name.text));
                self.put_outbox(&name, file);
                // Made inside a backup flow: unlocked at once with the
                // passphrase just typed twice, never leaving the flow.
                if self.flow_vault_made(&name, &first_phrase) {
                    self.toast(EN.vaults_vault_created);
                    return;
                }
                self.vaults.just_made = Some(name.clone());
                self.vaults.create = None;
                self.vaults.focus = None;
                self.screen = Screen::Vaults;
                self.toast(EN.vaults_vault_created);
                // Unlock, the new vault picked (FLOWS decision 10): its
                // passphrase is typed once more, then the flow it was made
                // for goes on, or its contents open.
                if let Some(i) = self.vault_files().iter().position(|f| f.name == name) {
                    self.vault_act(VaultAction::Open(i));
                }
            }
            Err(e) => {
                if let Some(c) = self.vaults.create.as_mut() {
                    c.error = Some(e.reason());
                }
            }
        }
    }

    /// `stem.ofv`, or the first `stem-n.ofv` no box holds.
    pub(crate) fn free_vault_name(&self, stem: &str) -> String {
        let taken = |n: &str| {
            self.outbox
                .iter()
                .chain(self.inbox.iter())
                .any(|i| i.name.eq_ignore_ascii_case(n))
        };
        let first = format!("{stem}.ofv");
        if !taken(&first) {
            return first;
        }
        (2..100)
            .map(|n| format!("{stem}-{n}.ofv"))
            .find(|n| !taken(n))
            .unwrap_or_else(|| format!("{stem}-100.ofv"))
    }

    /// Seals every open vault with changes into the Outbox (`PLAN.md`
    /// §5.3). A vault nobody changed is not rewritten: a slot's bytes
    /// change only when there is something to keep, which is all a second
    /// copy can compare (`docs/VAULT.md` §9). Before the session's seed
    /// arrived nothing is sealed: the lock that calls this asked first
    /// ([`Faraday::seal_waits`]).
    pub(crate) fn vault_seal_all(&mut self) {
        // What each open vault holds, remembered across the lock.
        self.vault_summaries_refresh();
        let mut sealed: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..self.vaults.open.len() {
            sealed.extend(self.vault_sealed(i));
        }
        for (name, bytes) in sealed {
            self.put_outbox(&name, bytes);
        }
        self.vaults.open.clear();
        self.vaults.load = None;
        self.vaults.pass_seeds = None;
    }

    /// Open vault `i`, with the signed-amount memory added, sealed: its
    /// name and bytes, or None when nobody changed it or the session's
    /// seed has not arrived.
    fn vault_sealed(&mut self, i: usize) -> Option<(String, Vec<u8>)> {
        // A held key's record says so before the vault goes
        // (`docs/NEW-WALLET.md` §14.3).
        self.held_sweep();
        // The signed-amount memory goes into every open vault that lacks
        // part of it, the oldest records leaving first when the slot is
        // full (`docs/VAULT.md` §7, type 10).
        if self.seal_amounts
            && let Some(v) = self.vaults.open.get_mut(i)
        {
            let have: Vec<[u8; 32]> = v
                .contents
                .of(kind::AMOUNTS)
                .filter_map(|(_, r)| crate::memory::from_record(r).map(|e| e.0))
                .collect();
            let new: Vec<Record> = self
                .signed_amounts
                .iter()
                .filter(|e| !have.contains(&e.0))
                .map(crate::memory::record_of)
                .collect();
            if !new.is_empty() {
                v.contents.records.extend(new);
                while v.contents.used() > v.header().slot_len as usize {
                    match v
                        .contents
                        .records
                        .iter()
                        .position(|r| r.kind == kind::AMOUNTS)
                    {
                        Some(k) => {
                            v.contents.records.remove(k);
                        }
                        None => break,
                    }
                }
                v.changes += 1;
            }
        }
        if self.vaults.open.get(i)?.changes == 0 {
            return None;
        }
        let draw = self.vault_draw(b"nonce")?;
        let mut nonce = [0u8; fv::NONCE_LEN];
        nonce.copy_from_slice(&draw[..fv::NONCE_LEN]);
        let v = &self.vaults.open[i];
        let bytes = v.opened.seal(&v.file, &v.contents, &nonce).ok()?;
        let name = v.name.clone();
        // The keys waiting for their copies that this sealing holds.
        self.flow_sealed(i, &name, &bytes);
        Some((name, bytes))
    }

    /// Locks open vault `i` alone: sealed into the Outbox when changed,
    /// what it holds remembered, and closed. The rest stay open. A
    /// changed vault stays open, its changes kept, while the session's
    /// seed has not arrived: then this returns false.
    pub(crate) fn vault_lock_one(&mut self, i: usize) -> bool {
        if i >= self.vaults.open.len() {
            return true;
        }
        if !self.seeded() && self.vaults.open[i].changes > 0 {
            self.commands
                .push_back(osk_shell_api::Command::RequestEntropy);
            self.toast(EN.seed_missing);
            return false;
        }
        self.vault_summaries_refresh();
        if let Some((name, bytes)) = self.vault_sealed(i) {
            self.put_outbox(&name, bytes);
        }
        self.vaults.open.remove(i);
        self.vaults.current = 0;
        self.vaults.item_open = false;
        self.vaults.form = None;
        true
    }

    /// Typing on a vault screen. Returns whether the key was taken.
    pub(crate) fn vault_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
        // The boot import's sheet has a passphrase field of its own.
        let import = self.sheet == Some(crate::Sheet::Import) && self.screen == Screen::Home;
        // So has the sheet of seeds kept with no passphrase (F62).
        let pass_seeds = self.sheet == Some(crate::Sheet::PassphraseSeeds);
        if !import
            && !pass_seeds
            && !matches!(
                self.screen,
                Screen::Unlock
                    | Screen::CreateVault
                    | Screen::VaultContents
                    | Screen::Family
                    | Screen::Codes
                    | Screen::BackupFlow
            )
        {
            return false;
        }
        let Some(focus) = self.vaults.focus else {
            return false;
        };
        // Nothing secret is typed while a stick is attached: the key is
        // taken and dropped.
        if !self.may_load_keys()
            && matches!(focus, Focus::Passphrase | Focus::Phrase(..) | Focus::Dice)
        {
            self.vaults.focus = None;
            return true;
        }
        if key == K::Escape {
            self.vaults.focus = None;
            // The backup flow's Make form is not left by Escape.
            if self.vaults.form.is_some() && self.screen != Screen::BackupFlow {
                self.vaults.form = None;
            }
            return true;
        }
        if focus == Focus::Dice {
            if let Some((_, rolls, _)) = self.vaults.dice.as_mut() {
                match key {
                    K::Char(c) if ('1'..='6').contains(&c) => rolls.text.push(c),
                    K::Backspace => {
                        rolls.text.pop();
                    }
                    K::Enter => self.vault_act(VaultAction::DiceUse),
                    _ => {}
                }
            }
            return true;
        }
        if focus == Focus::Prompt {
            if let Some(p) = self.vaults.prompt.as_mut() {
                match key {
                    K::Char(c) if !c.is_control() => p.text.text.push(c),
                    K::Backspace => {
                        p.text.text.pop();
                    }
                    K::Enter => self.vault_act(VaultAction::PromptGo),
                    _ => {}
                }
                if let Some(p) = self.vaults.prompt.as_mut() {
                    p.error = None;
                }
            }
            return true;
        }
        if key == K::Enter || key == K::Tab {
            match focus {
                Focus::Dice | Focus::Prompt | Focus::Second => {}
                Focus::Passphrase => {
                    if key == K::Enter && import {
                        self.import_act(crate::boot_import::ImportAction::Submit);
                    } else if key == K::Enter {
                        self.vault_act(VaultAction::Unlock);
                    }
                }
                Focus::Name => {
                    self.vaults.focus = self.may_load_keys().then_some(Focus::Phrase(0, false));
                }
                Focus::Phrase(i, second) => {
                    let n = self.vaults.create.as_ref().map_or(0, |c| c.phrases.len());
                    self.vaults.focus = if !second {
                        Some(Focus::Phrase(i, true))
                    } else if i + 1 < n {
                        Some(Focus::Phrase(i + 1, false))
                    } else if key == K::Enter {
                        self.vault_act(VaultAction::CGo);
                        None
                    } else {
                        Some(Focus::Phrase(0, false))
                    };
                }
                Focus::Field(k) => {
                    let n = self.vaults.form.as_ref().map_or(0, |f| f.fields.len());
                    let lines = self
                        .vaults
                        .form
                        .as_ref()
                        .is_some_and(|f| multiline(f.kind, f.fields[k].0));
                    if key == K::Enter && lines {
                        if let Some(b) = self.vault_box(focus) {
                            b.text.push('\n');
                        }
                    } else if key == K::Enter && (k + 1 >= n) {
                        self.vault_act(VaultAction::FormSave);
                    } else {
                        self.vaults.focus = Some(Focus::Field((k + 1) % n.max(1)));
                    }
                }
            }
            return true;
        }
        let Some(b) = self.vault_box(focus) else {
            return false;
        };
        match key {
            K::Char(c) if !c.is_control() => {
                b.text.push(c);
            }
            K::Backspace => {
                b.text.pop();
            }
            _ => return false,
        }
        if let Some(c) = self.vaults.create.as_mut() {
            c.error = None;
        }
        self.vaults.unlock_error = None;
        true
    }

    fn vault_box(&mut self, focus: Focus) -> Option<&mut TextBox> {
        match focus {
            Focus::Passphrase => Some(&mut self.vaults.passphrase),
            Focus::Phrase(i, second) => self
                .vaults
                .create
                .as_mut()?
                .phrases
                .get_mut(i)
                .map(|(a, b)| if second { b } else { a }),
            Focus::Field(k) => self
                .vaults
                .form
                .as_mut()?
                .fields
                .get_mut(k)
                .map(|f| &mut f.2),
            Focus::Dice => self.vaults.dice.as_mut().map(|d| &mut d.1),
            Focus::Prompt => self.vaults.prompt.as_mut().map(|p| &mut p.text),
            Focus::Name => self.vaults.create.as_mut().map(|c| &mut c.name),
            Focus::Second => Some(&mut self.vaults.apk.second),
        }
    }
}

/// Longest name a vault's file takes, before `.ofv`.
pub const NAME_MAX: usize = 40;

/// The file name, less `.ofv`, a vault named `typed` is written under:
/// letters, digits, hyphens and underscores, spaces made hyphens,
/// anything else left out; `vault` when that leaves nothing.
pub fn vault_stem(typed: &str) -> String {
    let mut out = String::new();
    for c in typed.trim().chars() {
        if out.chars().count() >= NAME_MAX {
            break;
        }
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c.is_whitespace() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        "vault".to_string()
    } else {
        out.to_string()
    }
}

/// The words a list's dice rolls spell, for as many whole words as the
/// rolls make (`osk-bip`'s diceware lists, in roll order).
pub fn dice_words(rolls: &str, list: osk_bip::diceware::List) -> Vec<&'static str> {
    let per = list.dice_per_word();
    let digits: Vec<u8> = rolls
        .bytes()
        .filter(|b| (b'1'..=b'6').contains(b))
        .map(|b| b - b'0')
        .collect();
    digits
        .chunks_exact(per)
        .filter_map(|w| list.word(w))
        .collect()
}

/// The place in its list of each whole word the rolls make, in order:
/// the dice read as a number in base 6, first die highest.
pub fn dice_word_indices(rolls: &str, list: osk_bip::diceware::List) -> Vec<u16> {
    let digits: Vec<u16> = rolls
        .bytes()
        .filter(|b| (b'1'..=b'6').contains(b))
        .map(|b| u16::from(b - b'1'))
        .collect();
    digits
        .chunks_exact(list.dice_per_word())
        .map(|w| w.iter().fold(0u16, |n, d| n * 6 + d))
        .collect()
}

/// The master seed a key record holds, when it holds a seed rather than
/// words.
pub fn seed_of(r: &Record) -> Option<Zeroizing<Vec<u8>>> {
    match osk_backup::oskb::payload_of(osk_backup::oskb::KIND_SEED, r.field(field::KEY)?)? {
        osk_backup::oskb::Opened::Seed(s) => Some(s),
        _ => None,
    }
}

/// Whether a form field takes several lines, Enter starting a new one: a
/// note's text and an entry's notes.
pub fn multiline(kind: u8, number: u8) -> bool {
    matches!(
        (kind, number),
        (kind::NOTE, field::NOTE) | (kind::ENTRY, field::NOTES)
    )
}

/// A mnemonic's words, space-separated, in wiped memory.
pub(crate) fn phrase_of(m: &osk_bip::bip39::Mnemonic) -> Zeroizing<String> {
    let lang = m.language();
    let mut out = crate::secret_text::room();
    for (k, &i) in m.indices().iter().enumerate() {
        if k > 0 {
            out.push(' ');
        }
        out.push_str(lang.word(i));
    }
    out
}

/// What the lock, write-out and power-off sheets list (§4.5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeptWiped {
    /// Kept: one line per vault with its currency, then the files For
    /// the stick.
    pub kept: Vec<String>,
    /// Wiped from memory, or lost at power-off: one line each, `true` for
    /// what is nowhere else.
    pub wiped: Vec<(String, bool)>,
}

/// A vault file's currency (`docs/SIMPLIFY.md` §3.5), computed from the
/// receipt's hashes, never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Currency {
    /// Sealed For the stick, and no receipt names it.
    NeverWritten,
    /// The last receipt that names it wrote the bytes it has now: on the
    /// stick of this label.
    Current(String),
    /// Open with unsaved changes, or sealed bytes that differ from the
    /// receipt's.
    Changed,
    /// Read from a stick and not changed since, the stick not known.
    Unchanged,
}

impl Currency {
    /// The line a person reads: "Never written", "On STICK · current",
    /// "Changed since written", "Unchanged".
    pub fn line(&self) -> String {
        match self {
            Currency::NeverWritten => EN.vaults_never_written.to_string(),
            Currency::Current(label) => fill(EN.vaults_on_current, &[&label.to_string()]),
            Currency::Changed => EN.vaults_changed_since.to_string(),
            Currency::Unchanged => EN.vaults_unchanged.to_string(),
        }
    }
}

/// What the app remembers of a vault seen open, across a lock
/// (`docs/VAULT.md` §11): names and fingerprints, never anything that
/// spends. Kept in the kept state, so it is gone at power-off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VaultSummary {
    /// The file it was opened from.
    pub file: String,
    /// The file's salt, which is the vault's identity.
    pub salt: [u8; 32],
    /// The open slot's name.
    pub name: String,
    /// Its wallets: name and shape.
    pub wallets: Vec<(String, String)>,
    /// Its wallets' descriptor checksums, in the same order.
    pub sums: Vec<String>,
    /// The descriptor checksums of the wallets it keeps a plan for.
    pub planned: Vec<String>,
    /// Its keys' fingerprints.
    pub keys: Vec<String>,
    /// How many entries it holds.
    pub entries: usize,
    /// Its backup maps' lines: where each place is and what it holds
    /// (record type 11, field 4).
    pub map: Vec<String>,
    /// When it was last seen open, Unix seconds, when the clock is known.
    pub seen: Option<u64>,
}

impl VaultSummary {
    /// The line a locked vault's row and Home's Unlock lead read under
    /// its name: "Savings 2 of 3 · key 9a6a2580 · 12 entries · seen
    /// 14:02". `tz_min` is the device's time zone (`docs/TOTP.md` §2.3).
    pub fn line(&self, tz_min: i16) -> String {
        let mut parts: Vec<String> = Vec::new();
        // One or two wallets by name and the shape's first part ("2 of
        // 3", "Single key"); more, by count.
        if self.wallets.len() <= 2 {
            for (name, shape) in &self.wallets {
                let short = shape.split(" · ").next().unwrap_or(shape);
                parts.push(format!("{name} {short}"));
            }
        } else {
            parts.push(fill(
                EN.vaults_wallets_n,
                &[&self.wallets.len().to_string()],
            ));
        }
        // The same for keys: one or two by fingerprint, more by count.
        match self.keys.len() {
            0 => {}
            1 => parts.push(fill(EN.vaults_key, &[&self.keys[0].to_string()])),
            2 => parts.push(fill(
                EN.vaults_keys_inline,
                &[&self.keys.join(", ").to_string()],
            )),
            n => parts.push(fill(EN.vaults_keys, &[&n.to_string()])),
        }
        match self.entries {
            0 => {}
            1 => parts.push(EN.vaults_1_entry.to_string()),
            n => parts.push(fill(EN.vaults_entries_n, &[&n.to_string()])),
        }
        if let Some(t) = self.seen {
            parts.push(fill(
                EN.vaults_seen,
                &[&time_of_day(local_secs(t, tz_min)).to_string()],
            ));
        }
        if parts.is_empty() {
            return EN.vaults_nothing_in_it.to_string();
        }
        parts.join(" · ")
    }
}

/// A time of day from local Unix-style seconds ([`local_secs`]), hh:mm.
pub fn time_of_day(unix_secs: u64) -> String {
    let s = unix_secs % 86_400;
    format!("{:02}:{:02}", s / 3600, (s / 60) % 60)
}

/// [`time_of_day`], with seconds: hh:mm:ss, for Settings' and Set the
/// time's Now row.
pub fn time_of_day_secs(unix_secs: u64) -> String {
    let s = unix_secs % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

/// One text field of the kept summaries: no tab or line break.
fn kept_field(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// A record type 11 map line (field 4) with a leading place name
/// replaced by "Place n": a vault written before place names moved out
/// of this field named the place there; `names` are the same record's
/// field 3 values, in place order. A line already naming the place by
/// number, or one for a vault, files, software or an away seed, is
/// returned unchanged.
fn redact_place_name(line: &str, names: &[String]) -> String {
    for (p, name) in names.iter().enumerate() {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(name).and_then(|r| r.strip_prefix(':')) {
            return fill(EN.vaults_place, &[&(p + 1).to_string(), rest]);
        }
    }
    line.to_string()
}

/// The summaries as the kept state holds them: a `vault` line per
/// vault, then its `wallet`, `key` and `map` lines, fields split by tabs.
pub(crate) fn summaries_encode(all: &[VaultSummary]) -> Vec<u8> {
    let mut out = String::new();
    for s in all {
        let salt: String = s.salt.iter().map(|b| format!("{b:02x}")).collect();
        let seen = s.seen.map(|t| t.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "vault\t{salt}\t{}\t{}\t{}\t{seen}\n",
            kept_field(&s.file),
            kept_field(&s.name),
            s.entries
        ));
        for (name, shape) in &s.wallets {
            out.push_str(&format!(
                "wallet\t{}\t{}\n",
                kept_field(name),
                kept_field(shape)
            ));
        }
        for k in &s.keys {
            out.push_str(&format!("key\t{}\n", kept_field(k)));
        }
        for c in &s.sums {
            out.push_str(&format!("sum\t{}\n", kept_field(c)));
        }
        for c in &s.planned {
            out.push_str(&format!("planned\t{}\n", kept_field(c)));
        }
        for m in &s.map {
            out.push_str(&format!("map\t{}\n", kept_field(m)));
        }
    }
    out.into_bytes()
}

/// The summaries read back; a line that does not read is skipped.
pub(crate) fn summaries_decode(bytes: &[u8]) -> Vec<VaultSummary> {
    let text = String::from_utf8_lossy(bytes);
    let mut out: Vec<VaultSummary> = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[..] {
            ["vault", salt, file, name, entries, seen] => {
                let mut s = VaultSummary::default();
                let bytes: Option<Vec<u8>> = (0..32)
                    .map(|i| {
                        salt.get(i * 2..i * 2 + 2)
                            .and_then(|h| u8::from_str_radix(h, 16).ok())
                    })
                    .collect();
                match bytes.and_then(|b| <[u8; 32]>::try_from(b).ok()) {
                    Some(b) if salt.len() == 64 => s.salt = b,
                    _ => continue,
                }
                s.file = file.to_string();
                s.name = name.to_string();
                s.entries = entries.parse().unwrap_or(0);
                s.seen = seen.parse().ok();
                out.push(s);
            }
            ["wallet", name, shape] => {
                if let Some(s) = out.last_mut() {
                    s.wallets.push((name.to_string(), shape.to_string()));
                }
            }
            ["key", fp] => {
                if let Some(s) = out.last_mut() {
                    s.keys.push(fp.to_string());
                }
            }
            ["sum", c] => {
                if let Some(s) = out.last_mut() {
                    s.sums.push(c.to_string());
                }
            }
            ["planned", c] => {
                if let Some(s) = out.last_mut() {
                    s.planned.push(c.to_string());
                }
            }
            ["map", m] => {
                if let Some(s) = out.last_mut() {
                    s.map.push(m.to_string());
                }
            }
            _ => {}
        }
    }
    out
}

/// A clock reading below this is not known: the Pi has no real-time
/// clock and starts at 1970 (`docs/TOTP.md` §1.1), the same floor
/// `opensigner/shells/pi/src/files.rs` uses for file dates.
pub const CLOCK_FLOOR: u64 = 1_577_836_800;

/// Unix seconds turned into local seconds for a time zone offset east of
/// UTC, in minutes (`docs/TOTP.md` §2.3). Every date or time drawn on
/// screen or written for the person goes through this; what goes into a
/// format (a signature's time, the TOTP counter) stays UTC.
pub fn local_secs(unix_secs: u64, tz_min: i16) -> u64 {
    (unix_secs as i64 + i64::from(tz_min) * 60).max(0) as u64
}

/// A time zone offset, minutes east of UTC, as `+02:00`, `-03:30` or
/// `+00:00` (`docs/TOTP.md` §2.1 and §2.2): the sign always written, two
/// digits each of hours and minutes.
pub fn tz_offset_text(tz_min: i16) -> String {
    let sign = if tz_min < 0 { '-' } else { '+' };
    let abs = tz_min.unsigned_abs();
    format!("{sign}{:02}:{:02}", abs / 60, abs % 60)
}

/// `s` as a time zone offset `tz_offset_text` writes: the sign, two
/// digits of hours, `:`, two digits of minutes a multiple of 15, in
/// range −12:00 to +14:00.
pub fn parse_tz_offset(s: &str) -> Option<i16> {
    let b = s.as_bytes();
    if b.len() != 6 || b[3] != b':' {
        return None;
    }
    let sign: i16 = match b[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hh: i16 = s.get(1..3)?.parse().ok()?;
    let mm: i16 = s.get(4..6)?.parse().ok()?;
    if !matches!(mm, 0 | 15 | 30 | 45) {
        return None;
    }
    let total = sign * (hh * 60 + mm);
    (-720..=840).contains(&total).then_some(total)
}

impl Faraday {
    /// Now on this computer's clock, Unix seconds: the last time the
    /// shell said, moved on by the ticks since, corrected by the offset
    /// a time set by hand gives it (`docs/TOTP.md` §1.2). With no offset,
    /// a reading below [`CLOCK_FLOOR`] is not known.
    pub fn clock_now(&self) -> Option<u64> {
        let at = self.vaults.unix_secs?;
        let raw = at + self.now_ms.saturating_sub(self.vaults.clock_at_ms) / 1000;
        match self.vaults.clock_offset {
            Some(off) => Some((raw as i64 + off).max(0) as u64),
            None => (raw >= CLOCK_FLOOR).then_some(raw),
        }
    }

    /// Set or Keep this time has been pressed this power-on
    /// (`docs/TOTP.md` §1.3); the desktop app counts as checked whenever
    /// its clock is known.
    pub fn clock_checked(&self) -> bool {
        self.vaults.clock_checked || (self.online && self.clock_now().is_some())
    }

    /// Whether the screen shows the clock or a TOTP code live: Settings'
    /// and Set the time's Now, the codes, an open vault's entries, whose
    /// detail has the Code row, and Sign with {key}'s time.
    pub(crate) fn shows_clock(&self) -> bool {
        // Sign with {key}'s signature time (F63).
        if self.sheet == Some(crate::Sheet::GpgSign) {
            return true;
        }
        match self.screen {
            Screen::Settings | Screen::SetTime | Screen::Codes => true,
            Screen::VaultContents => self.vaults.category == 2,
            _ => false,
        }
    }

    /// Writes each open vault's summary over the one kept for it: on
    /// unlock, after every change made on a vault screen, and as the
    /// session seals.
    pub(crate) fn vault_summaries_refresh(&mut self) {
        let seen = self.clock_now();
        let mut fresh = Vec::new();
        for v in &self.vaults.open {
            let wallets = v
                .contents
                .of(kind::WALLET)
                .map(|(i, r)| {
                    (
                        r.text(field::WALLET_NAME)
                            .unwrap_or(EN.vaults_wallet)
                            .to_string(),
                        v.contents
                            .described(i)
                            .map(|d| d.shape.clone())
                            .unwrap_or_default(),
                    )
                })
                .collect();
            let keys = v
                .contents
                .of(kind::KEY)
                .filter_map(|(_, r)| crate::vault_screens::key_fingerprint(self, r))
                .collect();
            let sum_of = |i: usize| v.contents.described(i).map(|d| d.checksum.clone());
            let sums = v
                .contents
                .of(kind::WALLET)
                .map(|(i, _)| sum_of(i).unwrap_or_default())
                .collect();
            let planned = v
                .contents
                .of(kind::PLAN)
                .filter_map(|(i, _)| sum_of(i))
                .collect();
            let map = v
                .contents
                .of(kind::PLAN)
                .flat_map(|(_, r)| {
                    let names: Vec<String> = r
                        .fields
                        .iter()
                        .filter(|f| f.number == field::PLAN_PLACE)
                        .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
                        .collect();
                    r.fields
                        .iter()
                        .filter(|f| f.number == field::PLAN_HOLDS)
                        .map(|f| redact_place_name(&String::from_utf8_lossy(&f.bytes), &names))
                        .collect::<Vec<_>>()
                })
                .collect();
            fresh.push(VaultSummary {
                file: v.name.clone(),
                salt: v.header().salt,
                name: v.label(),
                wallets,
                sums,
                planned,
                keys,
                entries: v.contents.of(kind::ENTRY).count(),
                map,
                seen,
            });
        }
        for s in fresh {
            match self.vaults.summaries.iter_mut().find(|o| o.salt == s.salt) {
                Some(o) => *o = s,
                None => self.vaults.summaries.push(s),
            }
        }
    }

    /// Where vault file `f` stands against the last write
    /// (`docs/SIMPLIFY.md` §3.5). A vault copied in from a stick,
    /// unchanged, that no receipt names is current on the stick it came
    /// from when that is known, else unchanged.
    pub fn currency(&self, f: &VaultFile) -> Option<Currency> {
        let changed = f
            .open
            .and_then(|o| self.vaults.open.get(o))
            .is_some_and(|v| v.changes > 0);
        let named = self.receipt.as_ref().filter(|r| r.wrote(&f.name));
        match named {
            Some(r) => {
                let boxes = if f.in_outbox {
                    &self.outbox
                } else {
                    &self.inbox
                };
                let same = boxes
                    .iter()
                    .find(|i| i.name == f.name)
                    .is_some_and(|i| r.holds(&i.name, &i.hash()));
                Some(if same && !changed {
                    Currency::Current(r.label.clone())
                } else {
                    Currency::Changed
                })
            }
            None if f.in_outbox => Some(Currency::NeverWritten),
            None if changed => Some(Currency::Changed),
            // Read from a stick and not changed: on the stick it came
            // from, as it is, when that stick is known.
            // A vault file's hash was taken when it came in
            // ([`crate::Item::sha`]): drawing asks this on every frame.
            None if !self.vault_from.iter().any(|(n, ..)| *n == f.name) => {
                Some(Currency::Unchanged)
            }
            None => {
                let hash = self
                    .inbox
                    .iter()
                    .find(|i| i.name == f.name)
                    .map(crate::Item::hash);
                Some(
                    self.vault_from
                        .iter()
                        .find(|(n, h, _)| *n == f.name && Some(*h) == hash)
                        .map_or(Currency::Unchanged, |(.., label)| {
                            Currency::Current(label.clone())
                        }),
                )
            }
        }
    }

    /// The first vault changed since it was written, by the name it goes
    /// by: Home's lead rule 6 and the session strip's Write out.
    pub fn vault_changed(&self) -> Option<String> {
        self.vault_files().iter().find_map(|f| {
            (self.currency(f) == Some(Currency::Changed)).then(|| {
                f.open
                    .and_then(|o| self.vaults.open.get(o))
                    .map(|v| v.label())
                    .or_else(|| self.vault_summary(f).map(|s| s.name.clone()))
                    .unwrap_or_else(|| f.name.clone())
            })
        })
    }

    /// What is remembered of a vault file, when it was seen open since
    /// power-on.
    pub fn vault_summary(&self, f: &VaultFile) -> Option<&VaultSummary> {
        self.vaults
            .summaries
            .iter()
            .find(|s| s.salt == f.header.salt)
    }
}
