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

use faraday_vault::records::{self, field, kind};
use faraday_vault::{self as fv, Contents, Cost, Header, Record};
use osk_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
use std::collections::BTreeSet;
use zeroize::Zeroizing;

use crate::{Action, Faraday, FileKind, Screen, flow};

/// The presets of `docs/VAULT.md` §3.1: name, memory in MiB, passes, in
/// order of memory.
pub const PRESETS: [(&str, u32, u32); 5] = [
    ("Light", 64, 3),
    ("Standard", 256, 3),
    ("Medium", 512, 3),
    ("Strong", 1024, 4),
    ("Maximum", 2048, 2),
];

/// The place after the presets that stands for a custom cost.
pub const CUSTOM: usize = PRESETS.len();

/// Create a vault's two sizes (`docs/SIMPLIFY.md` §3.1): the label, the
/// preset it takes, and the machines it is for besides this one, in
/// `MACHINES` order. The second is the default.
pub const SIZES: [(&str, usize, [bool; 5]); 2] = [
    ("PCs only", 2, [true, false, false, false, false]),
    (
        "PCs and a Raspberry Pi",
        0,
        [true, false, false, true, false],
    ),
];

/// The size a new vault starts on: PCs and a Raspberry Pi.
pub const DEFAULT_SIZE: usize = 1;

/// The machines a vault may be opened on, besides this one: label, short
/// name, memory shown, memory in MiB.
pub const MACHINES: [(&str, &str, &str, u32); 5] = [
    ("Other PCs", "other PCs", "4 GB", 4096),
    ("Raspberry Pi 5", "Pi 5", "4 GB", 4096),
    ("Raspberry Pi 4", "Pi 4", "2 GB", 2048),
    ("Raspberry Pi 3", "Pi 3", "1 GB", 1024),
    ("Pi Zero 2 W", "Pi Zero 2 W", "512 MB", 512),
];

/// A custom cost's memory choices, MiB.
pub const CUSTOM_MEMORY: [u32; 8] = [64, 128, 256, 512, 1024, 2048, 3072, 4096];
/// A custom cost's pass choices.
pub const CUSTOM_PASSES: [u32; 7] = [2, 3, 4, 5, 6, 8, 10];

/// The kinds a vault's contents are listed by, with the record types each
/// shows.
pub const CATEGORIES: [(&str, &[u8]); 6] = [
    ("Bitcoin keys", &[kind::KEY]),
    ("Wallets", &[kind::WALLET]),
    ("Entries", &[kind::ENTRY]),
    ("Notes", &[kind::NOTE, kind::SHEET]),
    ("GPG keys", &[kind::GPG]),
    ("Secure Boot", &[kind::SECURE_BOOT]),
];

/// The labels a new vault's slots start with, in passphrase order.
const SLOT_LABELS: [&str; 4] = ["Main", "Second", "Third", "Fourth"];

/// Bits of a passphrase from dice counted strong: six words of the EFF
/// long list (`docs/VAULT.md` §3.1).
pub const STRONG_BITS: u32 = 77;

/// The words this list needs for a strong passphrase, said with where the
/// lists come from: text, since Faraday is offline.
pub fn dice_aim(list: osk_bip::diceware::List) -> String {
    let n = (1..=20)
        .find(|&n| list.bits(n) >= STRONG_BITS)
        .unwrap_or(20);
    format!(
        "Aim for {n} words or more on this list: the count turns green at {} bits, which this \
         vault counts as strong. The lists and how to roll for them: eff.org/dice",
        STRONG_BITS
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
    /// A kind in the contents.
    Category(usize),
    /// An item of the kind.
    Item(usize),
    /// Held to show a secret field; does nothing when released.
    Reveal(usize),
    /// Held to delete the item.
    HoldDelete,
    /// A key loads at unlock, or stops loading.
    ToggleLoad,
    /// Load the item into the session.
    Load,
    /// Edit the item.
    Edit,
    /// Add an item of the current kind.
    Add,
    /// The contents' Add… row: every kind, each with its Add action.
    AddMenu,
    /// Add an item of kind n, from the Add… list.
    AddKind(usize),
    /// An empty vault's Put a wallet in it: Wallets.
    PutWallet,
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
    /// A revocation certificate for the selected GPG key to the Outbox.
    GpgRevoke,
    /// New self-signatures with the chosen expiry.
    GpgRenew,
    /// Choose an Inbox file for the selected GPG key to sign.
    GpgSignPick,
    /// Sign Inbox file n with the selected GPG key.
    GpgSign(usize),
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
}

/// Whether an action acts when held long enough rather than on release.
pub fn is_hold(a: Action) -> bool {
    matches!(
        a,
        Action::Vault(VaultAction::HoldDelete | VaultAction::SbHoldSign)
    )
}

/// What a one-line prompt in the contents is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// The passphrase of an Inbox backup, by Inbox position.
    Backup(usize),
    /// A BIP-39 passphrase to load the selected key with.
    KeyPassphrase,
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
    fn set(&mut self, s: &str) {
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

/// One open slot of one vault.
pub struct OpenVault {
    /// The file name it came under, and goes back under.
    pub name: String,
    /// The file as it was opened: the other slots are copied from it.
    file: Vec<u8>,
    opened: fv::Opened,
    /// The slot's records.
    pub contents: Contents,
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
            .unwrap_or("Vault")
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
    /// Make Secure Boot keys.
    SecureBoot,
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
    /// The contents show the Add… list: every kind with its Add action.
    pub add_menu: bool,
    /// The item shown in each kind.
    pub item: [usize; 6],
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
    /// Dice being rolled for a passphrase: which one, the rolls, and
    /// which of the three EFF lists.
    pub dice: Option<(usize, TextBox, osk_bip::diceware::List)>,
    /// The prompt in the contents.
    pub prompt: Option<Prompt>,
    /// This computer's clock, Unix seconds, as the shell last said.
    pub unix_secs: Option<u64>,
    /// When the shell last said it, on the app's own ticks, ms.
    pub(crate) clock_at_ms: u64,
    /// What is remembered of each vault seen open since power-on.
    pub summaries: Vec<VaultSummary>,
    /// The expiry chosen for a GPG key being made or renewed, in years;
    /// 0 for never.
    pub gpg_years: u32,
    /// Choosing an Inbox file for the selected GPG key to sign.
    pub signing: bool,
    /// What is typed into passphrase fields is shown in the clear.
    pub typed_shown: bool,
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
    /// SHA-256 of the key's payload: deriving one takes 2048 rounds.
    pub fingerprints: std::cell::RefCell<Vec<([u8; 32], String)>>,
    draws: u64,
}

/// Memory in MiB as a person reads it.
pub fn mib_text(mib: u32) -> String {
    if mib >= 1024 {
        if mib.is_multiple_of(1024) {
            format!("{} GiB", mib / 1024)
        } else {
            format!("{:.1} GiB", f64::from(mib) / 1024.0)
        }
    } else {
        format!("{mib} MiB")
    }
}

/// A slot size as a person reads it.
pub fn size_text(slot: u32) -> String {
    if slot >= 1_048_576 {
        format!("{} MiB", slot / 1_048_576)
    } else {
        format!("{} KiB", slot / 1024)
    }
}

/// A file size as a person reads it.
pub fn file_text(len: usize) -> String {
    if len >= 1_000_000 {
        format!("{:.1} MB", len as f64 / 1e6)
    } else {
        format!("{} KB", len.div_ceil(1000))
    }
}

/// A cost as a person reads it.
pub fn cost_text(cost: &Cost) -> String {
    format!(
        "{} · {} {}",
        mib_text(cost.memory_kib / 1024),
        cost.passes,
        if cost.passes == 1 { "pass" } else { "passes" }
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
            Some(s) if s < 10.0 => format!("about {s:.1} s"),
            Some(s) => format!("about {} s", s.round() as u64),
            None => "not measured yet".to_string(),
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

    /// The machine with the least memory among this one and those chosen:
    /// label and MiB.
    pub fn weakest(&self) -> (String, u32) {
        let here = self.memory_free_mib.unwrap_or(4096);
        let mut w = ("This computer".to_string(), here);
        if let Some(c) = self.create.as_ref() {
            for (k, m) in MACHINES.iter().enumerate() {
                if c.on[k] && m.3 < w.1 {
                    w = (m.0.to_string(), m.3);
                }
            }
        }
        w
    }

    /// The largest preset that fits the weakest machine.
    pub fn suggested(&self) -> usize {
        let (_, ram) = self.weakest();
        (0..PRESETS.len())
            .rev()
            .find(|&k| PRESETS[k].1 <= ram / 2)
            .unwrap_or(0)
    }

    /// The size row chosen, while the preset and machines are one of
    /// `SIZES`.
    pub fn size_row(&self) -> Option<usize> {
        let c = self.create.as_ref()?;
        SIZES
            .iter()
            .position(|(_, p, on)| c.preset == Some(*p) && c.on == *on)
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
    pub fn unsaved(&self) -> (Vec<String>, Vec<String>) {
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
                    .of(kind::WALLET)
                    .filter_map(|(_, r)| r.text(field::WALLET))
            })
            .filter_map(|t| crate::wallet::read_wallet(t).ok())
            .map(|p| p.to_descriptor())
            .collect();
        let keys = self
            .session
            .keys
            .iter()
            .filter(|k| {
                let payload = k
                    .words
                    .as_ref()
                    .and_then(|w| osk_bip::bip39::Mnemonic::parse(k.language, w).ok())
                    .map(|m| records::words_payload(&m));
                !payload.is_some_and(|p| held_keys.contains(&p.as_slice()))
            })
            .map(|k| k.label.clone())
            .collect();
        let wallets = self
            .session
            .wallets
            .iter()
            .filter(|w| !held_wallets.contains(&w.policy.to_descriptor()))
            .map(|w| w.name.clone())
            .collect();
        (keys, wallets)
    }

    /// What a lock, a write-out or a power-off keeps and what it wipes
    /// (`docs/SIMPLIFY.md` §4.5): each vault with its currency, the files
    /// For the stick, and each seed and wallet in memory with where else
    /// it is. At `power_off` a vault not written as it stands and the
    /// files For the stick are lost, not kept.
    pub fn kept_wiped(&self, power_off: bool) -> KeptWiped {
        let mut out = KeptWiped::default();
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
            if lost {
                out.wiped.push((line, true));
            } else {
                out.kept.push(line);
            }
        }
        let n = self.outbox.len();
        let files = format!(
            "{n} {} {}",
            if n == 1 { "file" } else { "files" },
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
        let in_vault_key = |payload: &[u8]| {
            self.vaults.open.iter().find_map(|v| {
                v.contents
                    .of(kind::KEY)
                    .any(|(_, r)| r.field(field::KEY) == Some(payload))
                    .then(|| v.label())
            })
        };
        let in_vault_wallet = |descriptor: &str| {
            self.vaults.open.iter().find_map(|v| {
                v.contents
                    .of(kind::WALLET)
                    .filter_map(|(_, r)| r.text(field::WALLET))
                    .filter_map(|t| crate::wallet::read_wallet(t).ok())
                    .any(|p| p.to_descriptor() == descriptor)
                    .then(|| v.label())
            })
        };
        let checked: &[[u8; 4]] = self.backup.as_ref().map_or(&[], |b| &b.checked);
        for k in &self.session.keys {
            let vault = k
                .words
                .as_ref()
                .and_then(|w| osk_bip::bip39::Mnemonic::parse(k.language, w).ok())
                .and_then(|m| in_vault_key(&records::words_payload(&m)));
            let copy = checked.contains(&k.master.fingerprint().0);
            let (state, nowhere) = match (vault, copy) {
                (Some(v), true) => (format!("in {v} · copy checked"), false),
                (Some(v), false) => (format!("in {v}"), false),
                (None, true) => ("copy checked".to_string(), false),
                (None, false) => ("nowhere else".to_string(), true),
            };
            // A key with no name of its own goes by "Seed {fingerprint}",
            // as the backup map writes it.
            let fp = crate::wallet::fp_text(k.master.fingerprint());
            let name = if k.label.is_empty() || k.label.eq_ignore_ascii_case(&fp) {
                format!("Seed {fp}")
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
            let (state, nowhere) = match in_vault_wallet(&w.policy.to_descriptor()) {
                Some(v) => (format!("in {v}"), false),
                None if in_files => ("in Files".to_string(), false),
                None => ("nowhere else".to_string(), true),
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
        let files = self.vault_files();
        let locked: Vec<usize> = (0..files.len())
            .filter(|&i| files[i].open.is_none())
            .collect();
        let (label, a) = match locked[..] {
            [] => ("Make a vault".to_string(), VaultAction::CreateFrom(back)),
            [k] => (
                format!("Unlock {}", files[k].name),
                VaultAction::OpenFrom(k, back),
            ),
            _ => ("Unlock a vault".to_string(), VaultAction::ListFrom(back)),
        };
        Some((label, crate::Action::Vault(a)))
    }

    /// The flow a vault is being made or unlocked for, by name: the way
    /// back's label and the "Then:" on Create a vault.
    pub fn back_name(&self, s: Screen) -> &'static str {
        match s {
            Screen::Create => "Create a wallet",
            Screen::Restore => "Load or restore a wallet",
            Screen::Family => "Spend",
            Screen::Backup => "Back up a wallet",
            Screen::Bip85 => "Child seeds and passwords",
            Screen::Lightning => "Lightning node key",
            Screen::Home if self.import.is_some() => "Import",
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
            "Tools"
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

    /// 32 bytes for a vault's seal nonce: the session's entropy, hashed
    /// with a counter and what they are for. A nonce has only to differ
    /// each time, and a seal at lock must not wait for the system; every
    /// key comes from [`Faraday::fresh`] instead.
    pub(crate) fn vault_draw(&mut self, what: &[u8]) -> [u8; 32] {
        self.vaults.draws += 1;
        let mut e = sha256::Hash::engine();
        e.input(b"faraday vault draw");
        e.input(what);
        e.input(&self.seed);
        e.input(&self.vaults.draws.to_le_bytes());
        sha256::Hash::from_engine(e).to_byte_array()
    }

    /// Runs a vault action.
    pub(crate) fn vault_act(&mut self, a: VaultAction) {
        use VaultAction as V;
        // A small panel's contents page changes to another page: it starts
        // at its top.
        if matches!(
            a,
            V::Item(_) | V::ItemBack | V::Add | V::AddMenu | V::Edit | V::Rename | V::Category(_)
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
                        self.toast(&format!(
                            "{} loaded from {}",
                            crate::wallet::fp_text(fp),
                            self.vaults.open[v].name
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
                    self.family_opened();
                }
                self.toast(&format!(
                    "{wallets} {} and {keys} {} loaded",
                    if wallets == 1 { "wallet" } else { "wallets" },
                    if keys == 1 { "seed" } else { "seeds" }
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
                    self.family_opened();
                }
                self.toast(&format!(
                    "{keys} {} and {wallets} {} loaded",
                    if keys == 1 { "seed" } else { "seeds" },
                    if wallets == 1 { "wallet" } else { "wallets" }
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
                if i < self.vaults.open.len() {
                    self.vaults.current = i;
                    self.vaults.form = None;
                    self.vaults.saving = false;
                    self.vaults.item_open = false;
                    self.vault_item_offset = 0.0;
                }
            }
            V::Category(k) => {
                self.vaults.category = k.min(CATEGORIES.len() - 1);
                self.vault_item_offset = 0.0;
                self.vaults.category_chosen = true;
                self.vaults.add_menu = false;
                self.vaults.form = None;
                self.vaults.saving = false;
                self.vaults.item_open = false;
            }
            V::Item(k) => {
                self.vaults.item[self.vaults.category] = k;
                self.vault_item_offset = 0.0;
                self.vaults.form = None;
                self.vaults.saving = false;
                self.vaults.item_open = true;
            }
            V::ItemBack => {
                self.vaults.item_open = false;
                self.vaults.form = None;
                self.vaults.prompt = None;
                self.vaults.saving = false;
                self.vaults.signing = false;
                self.vaults.sb_images = false;
                self.vaults.sb_sign = None;
                self.vaults.focus = None;
            }
            V::Reveal(_) => {}
            V::HoldDelete => self.vault_delete(),
            V::ToggleLoad => {
                if let Some(at) = self.vault_selected_index()
                    && let Some(v) = self.vaults.open.get_mut(self.vaults.current)
                    && let Some(r) = v.contents.records.get_mut(at)
                    && r.kind == kind::KEY
                {
                    let flags = r
                        .field(field::KEY_FLAGS)
                        .and_then(|f| f.first().copied())
                        .unwrap_or(0);
                    r.set(field::KEY_FLAGS, &[flags ^ records::LOAD_AT_UNLOCK]);
                    v.changes += 1;
                }
            }
            V::Load => self.vault_load_selected(),
            V::Edit => self.vault_form(true),
            V::AddMenu => {
                self.vaults.add_menu = true;
                self.vaults.form = None;
                self.vaults.saving = false;
                self.vaults.item_open = false;
            }
            V::AddKind(k) => {
                self.vault_act(V::Category(k));
                self.vault_act(V::Add);
            }
            V::PutWallet => {
                self.screen = if self.session.wallets.is_empty() {
                    Screen::Start
                } else {
                    Screen::Wallets
                };
            }
            V::WriteOut => self.screen = Screen::Files,
            V::Add => {
                let kinds = CATEGORIES[self.vaults.category].1;
                if kinds.contains(&kind::GPG) {
                    self.vaults.gpg_years = 2;
                    self.vaults.signing = false;
                }
                self.vaults.sb_images = false;
                self.vaults.sb_sign = None;
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
                        fields: vec![(field::LABEL, "Name", name, false)],
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
                    if let Some(c) = self.vaults.create.as_mut()
                        && let Some((a, b)) = c.phrases.get_mut(i)
                        && !words.is_empty()
                    {
                        let phrase = Zeroizing::new(words.join(""));
                        a.set(&phrase);
                        b.set(&phrase);
                        let bits = words.len() as f32 * list.bits_per_word();
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
                    self.toast("Unlock a vault first");
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
            V::GpgYears(y) => self.vaults.gpg_years = y,
            V::GpgExport => self.gpg_export(),
            V::GpgRevoke => self.gpg_revoke(),
            V::GpgRenew => self.gpg_renew(),
            V::GpgSignPick => {
                self.vaults.signing = !self.vaults.signing;
                self.vaults.form = None;
            }
            V::GpgSign(k) => self.gpg_sign(k),
            V::SbOwnOnly(on) => self.vaults.sb_own_only = on,
            V::SbEnrol => self.sb_enrol(),
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
        }
    }

    fn vault_import_entries(&mut self, files: &[usize]) {
        if self.vaults.open.is_empty() {
            self.toast("Unlock a vault first");
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
            self.toast("Those entries do not fit in this vault's slot size");
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
        self.toast(&format!(
            "{n} {} added",
            if n == 1 { "entry" } else { "entries" }
        ));
    }

    /// A TOTP setup code the camera read, into a new entry's form; text of
    /// several entries, into the vault.
    pub(crate) fn vault_scanned_entry(&mut self, text: &str) -> Result<(), String> {
        let t = text.trim();
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
            self.screen = Screen::VaultContents;
            return Ok(());
        }
        if fv::entries::looks_like_entries(t) {
            let records = fv::entries::parse(t);
            if let Some(v) = self.vaults.open.get_mut(self.vaults.current) {
                let n = records.len();
                v.contents.records.extend(records);
                v.changes += 1;
                self.vaults.category = 2;
                self.screen = Screen::VaultContents;
                self.toast(&format!(
                    "{n} {} added",
                    if n == 1 { "entry" } else { "entries" }
                ));
                return Ok(());
            }
        }
        Err("Not a TOTP setup code".to_string())
    }

    fn vault_prompt_go(&mut self) {
        let Some(p) = self.vaults.prompt.as_mut() else {
            return;
        };
        match p.purpose {
            Purpose::Backup(_) => {
                if p.text.text.is_empty() {
                    p.error = Some("Type the backup's passphrase".to_string());
                } else {
                    self.vaults.working = Some(Work::Backup);
                    self.vaults.drawn = false;
                }
            }
            Purpose::Kdbx => {
                if p.text.text.chars().count() < 8 {
                    p.error = Some("A passphrase of 8 characters or more".to_string());
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
                let Some(m) = records::words_of(r) else {
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
                        self.toast(&format!("Key {} is loaded", crate::wallet::fp_text(fp)));
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
                    .with(field::KEY_LABEL, label.as_bytes())
                    .with(field::KEY_FLAGS, &[records::LOAD_AT_UNLOCK]),
                0,
            ),
            Ok(Opened::Seed(seed)) => {
                let mut payload = Zeroizing::new(vec![0u8; 1 + osk_backup::oskb::MAX_SEED]);
                payload[0] = seed.len() as u8;
                payload[1..1 + seed.len()].copy_from_slice(&seed);
                (
                    Record::new(kind::KEY)
                        .with(field::KEY, &payload)
                        .with(field::KEY_LABEL, label.as_bytes())
                        .with(field::KEY_FLAGS, &[records::LOAD_AT_UNLOCK]),
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
                    osk_backup::oskb::Error::Passphrase => {
                        "That passphrase does not open this backup".to_string()
                    }
                    osk_backup::oskb::Error::Memory(mib) => {
                        format!("Needs {mib} MiB, which this computer cannot spare")
                    }
                    osk_backup::oskb::Error::Format => "Not a backup this build reads".to_string(),
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
        self.vault_push(record, &format!("{label} is in the vault"));
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
                Work::SecureBoot => self.sb_make(),
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
        redraw
    }

    fn vault_unlock_check(&mut self) {
        let files = self.vault_files();
        let Some(f) = files.get(self.vaults.pick) else {
            return;
        };
        let error = if !self.may_load_keys() {
            Some(format!("Remove the {} to unlock", self.medium.noun()))
        } else if self.vaults.passphrase.text.is_empty() {
            Some("Type the passphrase".to_string())
        } else {
            match self.vaults.memory_free_mib {
                Some(free) if f.header.memory_to_open_mib() > free => Some(format!(
                    "Needs {}. This computer has {} free.",
                    mib_text(f.header.memory_to_open_mib()),
                    mib_text(free)
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
                    contents,
                    changes: 0,
                    skip: Default::default(),
                    each_shown: false,
                });
                self.vaults.current = self.vaults.open.len() - 1;
                // Unlocked for a vault category from Tools: that category.
                // Otherwise the first kind it holds.
                self.vaults.add_menu = false;
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
                self.vaults.item = [0; 6];
                // Keys not marked to load start unchosen; every wallet
                // starts chosen. Nothing loads until the person says.
                let v = self.vaults.current;
                let skip = self.vaults.open[v]
                    .contents
                    .of(kind::KEY)
                    .filter(|(_, r)| {
                        !r.field(field::KEY_FLAGS).is_some_and(|f| {
                            f.first().is_some_and(|b| b & records::LOAD_AT_UNLOCK != 0)
                        })
                    })
                    .map(|(i, _)| i)
                    .collect();
                self.vaults.open[v].skip = skip;
                let records = self.vaults.open[v].contents.records.clone();
                self.amounts_from_vault(&records);
                // The vault just made opens on its contents, empty, with
                // what to do next.
                let made = self.vaults.just_made.take() == Some(self.vaults.open[v].name.clone());
                self.toast(&format!("{name} unlocked", name = self.vaults.open[v].name));
                self.vault_summaries_refresh();
                if for_import {
                    self.vaults.back_to = None;
                    let before = self.session.wallets.len();
                    let label = self.import.as_ref().map(|i| i.label.clone());
                    self.import_go();
                    let file = self.vaults.open[v].name.clone();
                    self.unlock_landed(before, &file, label.as_deref());
                    return;
                }
                // Unlocked from the Spend tab, or for it: everything loads.
                let back = self.vaults.back_to.take();
                if self.screen == Screen::Family || back == Some(Screen::Family) {
                    self.screen = Screen::Family;
                    self.family_unlocked(v);
                } else if back.is_none() && !made {
                    // The Vaults list's Unlock (`docs/NEW-WALLET.md`
                    // §11.1a): the wallets it holds load now, with their
                    // keys where it holds them, and it lands on Wallets
                    // as the boot sheet's Unlock does. A vault opened
                    // from inside a flow, or just made, keeps `back_to`
                    // or `made` and takes the branch below instead.
                    let before = self.session.wallets.len();
                    self.vault_load(v, true);
                    if self.session.wallets.len() > before {
                        let file = self.vaults.open[v].name.clone();
                        self.unlock_landed(before, &file, None);
                    } else {
                        self.screen = Screen::Files;
                    }
                } else {
                    self.screen = back.unwrap_or(if made {
                        Screen::VaultContents
                    } else {
                        Screen::Files
                    });
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

    /// After an unlock that loaded wallets (`docs/NEW-WALLET.md` §11.1,
    /// §11.1a: the boot sheet's Unlock or the Vaults list's): Wallets, on
    /// the first loaded that can sign here, else the first loaded, and a
    /// toast naming what loaded from where. One that loaded none stays
    /// where it landed.
    fn unlock_landed(&mut self, before: usize, file: &str, stick: Option<&str>) {
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
                    .any(|&i| self.session.wallets[i].source != "Vault") =>
            {
                s
            }
            _ => file,
        };
        let what = match loaded.len() {
            1 => self.session.wallets[first].name.clone(),
            n => format!("{n} wallets"),
        };
        self.toast(&format!("{what} loaded from {from}"));
    }

    /// The keys and wallets open vault `v` holds, each with whether the
    /// session has it already and whether it is chosen for the next load.
    pub fn vault_rows(&self, v: usize) -> Vec<VaultRow> {
        let Some(open) = self.vaults.open.get(v) else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        for (i, r) in open.contents.of(kind::WALLET) {
            let text = r.text(field::WALLET).unwrap_or("");
            let loaded = crate::wallet::read_wallet(text).is_ok_and(|p| {
                self.session.wallets.iter().any(|w| {
                    crate::wallet::same_wallet(&w.policy) == crate::wallet::same_wallet(&p)
                })
            });
            rows.push(VaultRow {
                record: i,
                name: r.text(field::WALLET_NAME).unwrap_or("Wallet").to_string(),
                detail: crate::vault_screens::wallet_shape(r).unwrap_or_default(),
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
                .any(|k| crate::wallet::fp_text(k.master.fingerprint()) == fp);
            rows.push(VaultRow {
                record: i,
                name: r.text(field::KEY_LABEL).unwrap_or("Seed").to_string(),
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
        for (i, r) in open.contents.of(kind::WALLET) {
            let Ok(policy) = crate::wallet::read_wallet(r.text(field::WALLET).unwrap_or("")) else {
                continue;
            };
            let name = r.text(field::WALLET_NAME).unwrap_or("Wallet").to_string();
            let wallet = crate::wallet::Wallet {
                name: name.clone(),
                policy,
                source: String::new(),
            };
            let fps: Vec<String> = wallet
                .policy
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
        let rec = open
            .contents
            .records
            .get(r)
            .filter(|x| x.kind == kind::WALLET)?;
        let policy = crate::wallet::read_wallet(rec.text(field::WALLET)?).ok()?;
        let want = crate::wallet::same_wallet(&policy);
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
            self.toast(&format!(
                "{keys} {} loaded",
                if keys == 1 { "seed" } else { "seeds" }
            ));
        }
        self.session
            .wallets
            .iter()
            .position(|w| crate::wallet::same_wallet(&w.policy) == want)
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
        let mut descs: Vec<(String, String)> = Vec::new();
        let mut seeds: Vec<(Zeroizing<Vec<u8>>, String)> = Vec::new();
        let mut passphrases: Vec<Zeroizing<String>> = Vec::new();
        for (i, r) in open.contents.of(kind::KEY) {
            if !set.contains(&i) {
                continue;
            }
            let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
            if let Some(m) = records::words_of(r) {
                words.push((phrase_of(&m), label));
                passphrases.push(Zeroizing::new(
                    r.text(field::KEY_PASSPHRASE).unwrap_or("").to_string(),
                ));
            } else if let Some(seed) = seed_of(r) {
                seeds.push((seed, label));
            }
        }
        for (i, r) in open.contents.of(kind::WALLET) {
            if !set.contains(&i) {
                continue;
            }
            if let Some(text) = r.text(field::WALLET) {
                descs.push((
                    text.to_string(),
                    r.text(field::WALLET_NAME).unwrap_or("Wallet").to_string(),
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
        for (text, name) in &descs {
            let _ = self.session.add_wallet(name, text, "Vault");
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
                p.error = Some("No randomness from the system yet. Try again".to_string());
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
        let title = r.text(field::TITLE).unwrap_or("Entry").to_string();
        let mut notes = String::new();
        for (label, f) in [("Username", field::USERNAME), ("URL", field::URL)] {
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
                self.toast(&format!("{name} waits {}, sealed", self.medium.for_the()));
            }
            None => {
                if let Some(p) = self.vaults.prompt.as_mut() {
                    p.error = Some("That entry is too large for a KeePass export".to_string());
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
            self.toast("Deleted from the vault");
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
                let added = match records::words_of(r) {
                    Some(m) => {
                        let p = r.text(field::KEY_PASSPHRASE).unwrap_or("").to_string();
                        self.session.add_mnemonic(&m, &p, &label, None)
                    }
                    None => match seed_of(r) {
                        Some(seed) => self.session.add_seed(&seed, &label),
                        None => return,
                    },
                };
                match added {
                    Ok(fp) => self.toast(&format!("Key {} is loaded", crate::wallet::fp_text(fp))),
                    Err(e) => self.toast(&e.text()),
                }
            }
            kind::WALLET => {
                let text = r.text(field::WALLET).unwrap_or("").to_string();
                let name = r.text(field::WALLET_NAME).unwrap_or("Wallet").to_string();
                match self.session.add_wallet(&name, &text, "Vault") {
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
                (field::TITLE, "Title", false),
                (field::USERNAME, "Username", false),
                (field::PASSWORD, "Password", true),
                (field::URL, "URL", false),
                (field::NOTES, "Notes", false),
                (field::TOTP, "TOTP secret", true),
            ],
            kind::NOTE => &[(field::NOTE, "Text", false)],
            kind::GPG => &[(1, "Name", false), (2, "Email", false)],
            kind::SECURE_BOOT => &[(1, "Owner name", false)],
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
        let Some(form) = self.vaults.form.take() else {
            return;
        };
        self.vaults.focus = None;
        if form.kind == kind::SECURE_BOOT {
            if self.vaults.unix_secs.is_none() {
                self.vaults.form = Some(form);
                self.toast("This computer's clock is not known");
                return;
            }
            self.vaults.sb_owner = form.fields[0].2.text.trim().to_string();
            self.vaults.working = Some(Work::SecureBoot);
            self.vaults.drawn = false;
            return;
        }
        if form.kind == kind::GPG {
            let name = form.fields[0].2.text.to_string();
            let email = form.fields[1].2.text.to_string();
            if let Err(e) = self.gpg_make(&name, &email) {
                self.vaults.form = Some(form);
                self.toast(&e);
            } else {
                let n = self
                    .vaults
                    .open
                    .get(self.vaults.current)
                    .map_or(0, |v| v.contents.of(kind::GPG).count());
                self.vaults.item[self.vaults.category] = n.saturating_sub(1);
                self.toast_out("Made · the certificate and a revocation certificate");
            }
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
                "The note is empty"
            } else {
                "An entry needs a title"
            };
            self.vaults.form = Some(form);
            self.toast(what);
            return;
        }
        let mut record = match form.record.and_then(|i| v.contents.records.get(i)) {
            Some(r) => r.clone(),
            None => Record::new(form.kind),
        };
        for (n, _, b, _) in &form.fields {
            record.set(*n, b.text.as_bytes());
        }
        // A new record must still fit in the slot.
        let mut trial = v.contents.clone();
        match form.record {
            Some(i) if i < trial.records.len() => trial.records[i] = record.clone(),
            _ => trial.records.push(record.clone()),
        }
        if trial.used() > v.header().slot_len as usize {
            self.vaults.form = Some(form);
            self.toast("That does not fit in this vault's slot size");
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
            self.toast("Unlock a vault first");
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
                    format!("{} is in the vault as a note", it.name),
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
                        .with(field::KEY_LABEL, stem.as_bytes())
                        .with(field::KEY_FLAGS, &[records::LOAD_AT_UNLOCK]),
                    format!("{} is in the vault as a key", it.name),
                    0,
                )
            }
            FileKind::Wallet => (
                Record::new(kind::WALLET)
                    .with(field::WALLET, text.trim().as_bytes())
                    .with(field::WALLET_NAME, stem.as_bytes()),
                format!("{} is in the vault as a wallet", it.name),
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
            self.vault_push(record, &format!("{label} is in the vault"));
        }
    }

    /// Session key `k` as the vault's key record, loaded at unlock, and
    /// its label: what Save on Vaults writes, and the secret sheet for a
    /// seed. With its BIP-39 passphrase only when `with_passphrase`.
    pub(crate) fn key_record(
        &mut self,
        k: usize,
        with_passphrase: bool,
    ) -> Option<(Record, String)> {
        let key = self.session.keys.get(k)?;
        let Some(words) = key.words.as_ref() else {
            self.toast("This key has no words to keep");
            return None;
        };
        let m = osk_bip::bip39::Mnemonic::parse(key.language, words).ok()?;
        // Made here and its backup not done: the vault says so, beside
        // the seed.
        let flags = if self.key_held(key.master.fingerprint()) {
            records::LOAD_AT_UNLOCK | records::BACKUP_PENDING
        } else {
            records::LOAD_AT_UNLOCK
        };
        let mut record = Record::new(kind::KEY)
            .with(field::KEY, &records::words_payload(&m))
            .with(field::KEY_LABEL, key.label.as_bytes())
            .with(field::KEY_FLAGS, &[flags]);
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
        let text = match (w.policy.record(), w.policy.silent()) {
            (Some(r), _) => r.to_text(),
            (None, Some(s)) => s.to_text(),
            (None, None) => w.policy.to_descriptor_checksummed(),
        };
        let record = Record::new(kind::WALLET)
            .with(field::WALLET, text.as_bytes())
            .with(field::WALLET_NAME, w.name.as_bytes());
        let name = w.name.clone();
        self.vault_push(record, &format!("{name} is in the vault"));
    }

    pub(crate) fn vault_push(&mut self, record: Record, said: &str) {
        let Some(v) = self.vaults.open.get_mut(self.vaults.current) else {
            return;
        };
        let mut trial = v.contents.clone();
        trial.records.push(record.clone());
        if trial.used() > v.header().slot_len as usize {
            self.toast("That does not fit in this vault's slot size");
            return;
        }
        v.contents.records.push(record);
        v.changes += 1;
        self.toast(said);
    }

    fn vault_create_check(&mut self) {
        let free = self.vaults.memory_free_mib;
        let cost = self.vaults.form_cost();
        let medium = self.medium;
        let Some(c) = self.vaults.create.as_mut() else {
            return;
        };
        let error = if !self.sticks.is_empty() {
            Some(format!("Remove the {} first", medium.noun()))
        } else if let Some(n) = c
            .steps()
            .iter()
            .position(|&i| !c.done[i as usize] && i != vstep::PHRASES)
        {
            c.open = Some(c.steps()[n]);
            Some(format!("Finish step {} first", n + 1))
        } else if c.phrases.iter().any(|(a, _)| a.text.is_empty()) {
            c.open = Some(vstep::PHRASES);
            Some("Every passphrase needs a value".to_string())
        } else if (0..c.phrases.len()).any(|i| !Vaults::phrase_ok(c, i)) {
            c.open = Some(vstep::PHRASES);
            Some("Each passphrase must be typed the same twice".to_string())
        } else if (0..c.phrases.len())
            .any(|i| (i + 1..c.phrases.len()).any(|j| *c.phrases[i].0.text == *c.phrases[j].0.text))
        {
            c.open = Some(vstep::PHRASES);
            Some("Two passphrases are the same. Each must differ".to_string())
        } else if let (Some(free), Some((_, mem, _))) = (free, cost)
            && mem + 100 > free
        {
            Some("Choose a cost this computer can allocate".to_string())
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
                c.error = Some("No randomness from the system yet. Try again".to_string());
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
        let phrases: Vec<&[u8]> = c.phrases.iter().map(|(a, _)| a.text.as_bytes()).collect();
        let contents: Vec<Contents> = (0..phrases.len())
            .map(|i| Contents {
                records: vec![
                    Record::new(kind::SLOT_LABEL).with(field::LABEL, SLOT_LABELS[i].as_bytes()),
                ],
            })
            .collect();
        let made = fv::create(c.slot, cost, &phrases, &contents, &seed);
        match made {
            Ok(file) => {
                let name = self.free_vault_name(&vault_stem(&c.name.text));
                self.put_outbox(&name, file);
                self.vaults.just_made = Some(name.clone());
                self.vaults.create = None;
                self.vaults.focus = None;
                self.screen = Screen::Vaults;
                self.toast("Vault created");
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
    /// copy can compare (`docs/VAULT.md` §9).
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
    }

    /// Open vault `i`, with the signed-amount memory added, sealed: its
    /// name and bytes, or None when nobody changed it.
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
        let draw = self.vault_draw(b"nonce");
        let mut nonce = [0u8; fv::NONCE_LEN];
        nonce.copy_from_slice(&draw[..fv::NONCE_LEN]);
        let v = &self.vaults.open[i];
        let bytes = v.opened.seal(&v.file, &v.contents, &nonce).ok()?;
        Some((v.name.clone(), bytes))
    }

    /// Locks open vault `i` alone: sealed into the Outbox when changed,
    /// what it holds remembered, and closed. The rest stay open.
    pub(crate) fn vault_lock_one(&mut self, i: usize) {
        if i >= self.vaults.open.len() {
            return;
        }
        self.vault_summaries_refresh();
        if let Some((name, bytes)) = self.vault_sealed(i) {
            self.put_outbox(&name, bytes);
        }
        self.vaults.open.remove(i);
        self.vaults.current = 0;
        self.vaults.item_open = false;
        self.vaults.form = None;
    }

    /// Typing on a vault screen. Returns whether the key was taken.
    pub(crate) fn vault_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
        // The boot import's sheet has a passphrase field of its own.
        let import = self.sheet == Some(crate::Sheet::Import) && self.screen == Screen::Home;
        if !import
            && !matches!(
                self.screen,
                Screen::Unlock | Screen::CreateVault | Screen::VaultContents | Screen::Family
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
            if self.vaults.form.is_some() {
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
                Focus::Dice | Focus::Prompt => {}
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

    /// Empties the vault field typing goes to.
    pub(crate) fn vault_clear_focused(&mut self) {
        if let Some(f) = self.vaults.focus
            && let Some(b) = self.vault_box(f)
        {
            b.clear();
        }
        self.vaults.unlock_error = None;
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
fn phrase_of(m: &osk_bip::bip39::Mnemonic) -> Zeroizing<String> {
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
            Currency::NeverWritten => "Never written".to_string(),
            Currency::Current(label) => format!("On {label} · current"),
            Currency::Changed => "Changed since written".to_string(),
            Currency::Unchanged => "Unchanged".to_string(),
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
    /// 14:02".
    pub fn line(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        // One or two wallets by name and the shape's first part ("2 of
        // 3", "Single key"); more, by count.
        if self.wallets.len() <= 2 {
            for (name, shape) in &self.wallets {
                let short = shape.split(" · ").next().unwrap_or(shape);
                parts.push(format!("{name} {short}"));
            }
        } else {
            parts.push(format!("{} wallets", self.wallets.len()));
        }
        // The same for keys: one or two by fingerprint, more by count.
        match self.keys.len() {
            0 => {}
            1 => parts.push(format!("key {}", self.keys[0])),
            2 => parts.push(format!("keys {}", self.keys.join(", "))),
            n => parts.push(format!("{n} keys")),
        }
        match self.entries {
            0 => {}
            1 => parts.push("1 entry".to_string()),
            n => parts.push(format!("{n} entries")),
        }
        if let Some(t) = self.seen {
            parts.push(format!("seen {}", time_of_day(t)));
        }
        if parts.is_empty() {
            return "Nothing in it".to_string();
        }
        parts.join(" · ")
    }
}

/// A time of day from Unix seconds, hh:mm, as the computer's clock keeps
/// it (UTC on Faraday).
pub fn time_of_day(unix_secs: u64) -> String {
    let s = unix_secs % 86_400;
    format!("{:02}:{:02}", s / 3600, (s / 60) % 60)
}

/// One text field of the kept summaries: no tab or line break.
fn kept_field(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
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

impl Faraday {
    /// Now on this computer's clock, Unix seconds: the last time the
    /// shell said, moved on by the ticks since.
    pub(crate) fn clock_now(&self) -> Option<u64> {
        let at = self.vaults.unix_secs?;
        Some(at + self.now_ms.saturating_sub(self.vaults.clock_at_ms) / 1000)
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
                .map(|(_, r)| {
                    (
                        r.text(field::WALLET_NAME).unwrap_or("Wallet").to_string(),
                        crate::vault_screens::wallet_shape(r).unwrap_or_default(),
                    )
                })
                .collect();
            let keys = v
                .contents
                .of(kind::KEY)
                .filter_map(|(_, r)| crate::vault_screens::key_fingerprint(self, r))
                .collect();
            let sum_of =
                |text: Option<&str>| crate::wallet::read_wallet(text?).ok().map(|p| p.checksum());
            let sums = v
                .contents
                .of(kind::WALLET)
                .map(|(_, r)| sum_of(r.text(field::WALLET)).unwrap_or_default())
                .collect();
            let planned = v
                .contents
                .of(kind::PLAN)
                .filter_map(|(_, r)| sum_of(r.text(field::PLAN_WALLET)))
                .collect();
            let map = v
                .contents
                .of(kind::PLAN)
                .flat_map(|(_, r)| {
                    r.fields
                        .iter()
                        .filter(|f| f.number == field::PLAN_HOLDS)
                        .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
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
                    .is_some_and(|i| r.holds(&i.name, &i.bytes));
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
            None => {
                let hash = self
                    .inbox
                    .iter()
                    .find(|i| i.name == f.name)
                    .map(|i| crate::sha256_of(&i.bytes));
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
