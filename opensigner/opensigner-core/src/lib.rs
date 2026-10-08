//! The OpenSigner application (`docs/PLANNING.md` §4.1): screens, flows
//! and wording as a state machine. Shell [`Event`]s come in, [`Command`]s
//! and pixels go out; the shell never sees anything else.
//!
//! Milestones 3b, 5, 6, 7, 8 and 9 cover Home, Keys, the Load and Create
//! wizards, Key detail with its backup screens, quiz and SeedQR export,
//! Settings, the start-up self-test ([`selftest`]), the Sign flow
//! ([`sign`]) with its QR output, the scanner ([`scan`]) that routes any
//! QR or file by payload (UX.md §4), Verify › address ([`verify`]), the
//! wallet export for coordinators, session security ([`session`]): the
//! session PIN and lock screen, auto-lock and auto-wipe timers, and
//! wipe-and-exit; and Explore ([`explore`]), the live BIP-39/BIP-32
//! workspace. Learn is a tile on Home, a Menu of pages behind it. On the
//! `wide` size class the hub is a sidebar beside the same content
//! (UX.md §4) and every wizard or document is a capped column.
//!
//! Secrets: the wizards' words, entropy, passphrase and PIN live in
//! [`load`], [`create`], [`finish`], [`session`], [`quiz`], [`backup`],
//! [`pass_entry`] (the backup passphrase) and [`explore`], files the
//! secret lint keeps free of heap text, as it keeps the formats they are
//! sealed under (`osk-backup`, `osk-keep`). Loaded keys
//! are [`load::LoadedKey`]s: the seed and the words are sealed under the
//! session key (`docs/PLANNING.md` §5.1, §16.21), and the [`MasterKey`]
//! exists only while the session is unlocked, borrowed only while a PSBT
//! is signed. Locking drops every master key and the derived caches;
//! unlocking rebuilds them from the sealed seed. Everything this module
//! derives for display (fingerprints, addresses, extended public keys,
//! descriptors, PSBT inspections) is public data; the PSBT itself is
//! private data and is dropped when the Sign flow is left.
//!
//! Dropping the [`OpenSigner`] zeroizes everything it holds: the session
//! key, the PIN hash, every sealed value and every live master key erase
//! themselves through their `Drop` impls (asserted by `ZeroizeOnDrop`
//! bounds where the types are ours). What cannot be guaranteed is inside
//! rust-bitcoin: `Xpriv` is `Copy`, so derivation leaves copies of
//! intermediate private keys on the stack that only later stack use
//! overwrites, and libsecp256k1's context scratch space is its own.
//! `MasterKey` and `DerivedKey` erase the copies they own.

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use osk_bip::account::{AccountXpub, MultisigAccountXpub};
use osk_bip::bip39::{Language, MAX_WORDS, Mnemonic};
use osk_bip::bitcoin::secp256k1::PublicKey;
use osk_bip::bsms;
use osk_bip::diceware::List;
use osk_bip::frost::SecShare;
use osk_bip::keys::{
    DerivationPath, Fingerprint, MasterKey, MultisigScriptType, Network, ScriptType, Xpub,
};
use osk_bip::policy::{PolicyKey, Template, WalletPolicy};
use osk_bip::threshold::MAX_PARTICIPANTS as THRESHOLD_MAX_SHARES;
use osk_codec::{PayloadKind, classify, seedqr, ur};
use osk_crypto::{MnemonicBytes, SealedBytes, Secret, SeedBytes};
use osk_psbt::{Inspection, KeyRef, ShareKey};
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, Event, FileEntry, FileKind, Frame, Key, SecureHardware,
};
use osk_ui::canvas::Canvas;
use osk_ui::color::Theme;
use osk_ui::components::Unit;
use osk_ui::geom::{Metrics, Rect, Scale, Size, SizeClass};
use osk_ui::layout::{self, Layout, LayoutCtx, Node};
use osk_ui::organisms::SidebarItem;
use osk_ui::screens::Chrome;
use osk_ui::state::{Action, UiState};
use osk_ui::widgets::keyboard::{self, KeyInput};
use osk_ui::widgets::{self, HitTarget, Icon, Widget, tokens};
use zeroize::{Zeroize, Zeroizing};

pub mod backup;
pub mod build;
pub mod codes;
pub mod codex32;
pub mod create;
pub mod dice;
pub mod explore;
pub mod finish;
pub mod ids;
mod inspect;
pub mod keep;
mod learn_map;
pub mod lightning;
pub mod load;
pub mod message;
pub mod notes;
pub mod pass_entry;
pub mod quiz;
pub mod scan;
pub mod session;
pub mod settings;
pub mod shares;
pub mod sign;
pub mod silent;
pub mod strings;
mod text;
pub mod threshold;
pub mod tools;
pub mod vanity;
pub mod verify;
mod views;
pub mod wordlist;

use backup::{BackupFlow, BackupStep};
use build::WalletWizard;
use codex32::{Codex32Plan, Next as Codex32Next, Step as Codex32Step};
use create::CreateWizard;
use dice::DicePassphrase;
use explore::{Explore, Source};
use finish::{Finish, Material};
use ids::Id;
use inspect::{InspectDoc, Swap};
use lightning::Lightning;
use load::{EntryList, LoadWizard, LoadedKey, Step};
use quiz::{Quiz, QuizState};
use scan::{CameraRotation, Expect, ScanStage, ScanState};
use session::{Session, Unlock};
use shares::{Next as ShareNext, SharePlan, Step as ShareStep};
use sign::{QrMode, Save, SignFlow, Stage};
use strings::{EN, Strings};
use threshold::{COUNTS, Dealer};
use tools::{Calculator, CompareTransactions, Tool};
use verify::{AddressResult, SEARCH_DEPTH, VerifyStage, VerifyState};
use wordlist::WordList;

/// How much the platform can be trusted (`docs/PLANNING.md` §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssuranceTier {
    /// DIY device running nothing else.
    A,
    /// Phone with a secure element.
    B,
    /// Desktop.
    C,
    /// Browser.
    D,
}

impl AssuranceTier {
    /// Every tier, in order, for the tiers Document.
    pub const ALL: [AssuranceTier; 4] = [
        AssuranceTier::A,
        AssuranceTier::B,
        AssuranceTier::C,
        AssuranceTier::D,
    ];

    /// Parses `A`–`D` (any case).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "A" => Some(AssuranceTier::A),
            "B" => Some(AssuranceTier::B),
            "C" => Some(AssuranceTier::C),
            "D" => Some(AssuranceTier::D),
            _ => None,
        }
    }

    /// §4.8 Tier badge: "Tier C · desktop" on Home's status line and in
    /// About.
    pub fn badge(self, s: &Strings) -> &'static str {
        match self {
            AssuranceTier::A => s.tier_a_badge,
            AssuranceTier::B => s.tier_b_badge,
            AssuranceTier::C => s.tier_c_badge,
            AssuranceTier::D => s.tier_d_badge,
        }
    }

    /// The same badge on a 268 dp status line, where §4.8 drops the word
    /// "Tier" because the full form does not fit beside the network
    /// badge and two buttons.
    pub fn badge_short(self, s: &Strings) -> &'static str {
        match self {
            AssuranceTier::A => s.tier_a_badge_short,
            AssuranceTier::B => s.tier_b_badge_short,
            AssuranceTier::C => s.tier_c_badge_short,
            AssuranceTier::D => s.tier_d_badge_short,
        }
    }

    /// The letter alone, which is all the 268 dp status line has room
    /// for while the session badge shares it (§4.8).
    pub fn badge_letter(self, s: &Strings) -> &'static str {
        match self {
            AssuranceTier::A => s.tier_a_badge_letter,
            AssuranceTier::B => s.tier_b_badge_letter,
            AssuranceTier::C => s.tier_c_badge_letter,
            AssuranceTier::D => s.tier_d_badge_letter,
        }
    }

    /// Name with the platform: `Tier C · desktop`.
    pub fn name(self, s: &Strings) -> &'static str {
        match self {
            AssuranceTier::A => s.tier_a,
            AssuranceTier::B => s.tier_b,
            AssuranceTier::C => s.tier_c,
            AssuranceTier::D => s.tier_d,
        }
    }

    /// The one-paragraph statement the tiers Document shows under the
    /// tier's heading (UX.md A1: what the OS can and cannot see).
    pub fn statement(self, s: &Strings) -> &'static str {
        match self {
            AssuranceTier::A => s.tier_a_statement,
            AssuranceTier::B => s.tier_b_statement,
            AssuranceTier::C => s.tier_c_statement,
            AssuranceTier::D => s.tier_d_statement,
        }
    }
}

/// What the shell knows about the build it is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildInfo {
    /// The package version.
    pub version: &'static str,
    /// Hash of the core, when a reproducible build recorded one.
    pub core_hash: Option<&'static str>,
}

/// Which screen is showing, for tests and shells. Carries no state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenKind {
    /// The hub.
    Home,
    /// The first run's document, which a device with nothing loaded
    /// opens on until it has been left once.
    StartHere,
    /// The Result after the first key a person creates.
    Created,
    /// The list of loaded keys.
    Keys,
    /// The Choice that says which loaded key a flow starts from.
    PickKey,
    /// The list of registered wallets.
    Wallets,
    /// The menu of the four ways a key arrives.
    Add,
    /// The menu of the kinds of wallet.
    AddWallet,
    /// One wallet's members, as the Keys screen filtered to them.
    WalletKeys,
    /// One key's menu of screens.
    KeyDetail,
    /// One wallet in use: its menu of screens.
    Wallet,
    /// The field a wallet's name is typed in.
    WalletName,
    /// The address explorer of one key or one wallet.
    Addresses,
    /// One key's backup menu.
    BackupMenu,
    /// Typing the passphrase that opens a passphrase key of a loaded
    /// key's words.
    OpenPassphrase,
    /// Choosing the word count and index of a BIP-85 child seed.
    OpenChild,
    /// One of BIP-85's other applications: the length or the index on
    /// the digits pad, then the derived value on a Secret screen
    /// (`docs/PLANNING.md` §16.114).
    Bip85,
    /// The Result after a passphrase key or a BIP-85 child seed was
    /// opened from a loaded key.
    Opened,
    /// Tools › Lightning node key: the passphrase, the node's public
    /// key and the secret behind it (`docs/PLANNING.md` §16.116).
    Lightning,
    /// A key's vanity address grinder: the script type, the prefix, the
    /// run and its find (`docs/PLANNING.md` §16.117).
    Vanity,
    /// The hold-to-forget screen.
    Forget,
    /// The hold that keeps a key on the device.
    Keep,
    /// The pad that opens the key kept on the device: the first screen
    /// of a device that keeps one, and the screen whenever no key is
    /// loaded (`front_door`).
    StoredKey,
    /// The pad that sets the duress PIN, twice.
    DuressPin,
    /// The Result after the stored key was removed.
    KeptRemoved,
    /// The Load wizard.
    Load,
    /// The wallet builder.
    Build,
    /// The Create wizard.
    Create,
    /// The Backup flow over a loaded key: quiz, words, numbers.
    Backup,
    /// Network, unit, session, About, wipe.
    Settings,
    /// A Setting Choice over Settings: network, unit, a timer.
    Setting,
    /// What this build is: version, tier, core hash, self-test.
    About,
    /// The Sign flow: entry document or wizard.
    Sign,
    /// The files a shell listed for a file request.
    Files,
    /// The scanner.
    Scan,
    /// Verify › address.
    Verify,
    /// The message being signed: the text, the hold and the result.
    SignMessage,
    /// The answer for a signed message that arrived to be checked.
    CheckedMessage,
    /// The export of one key or one wallet, for a coordinator.
    Export,
    /// A read-only document for a scanned descriptor, key or text.
    Inspect,
    /// The Explore workspace.
    Explore,
    /// The list of tools, of which the key explorer is the first.
    Tools,
    /// Tools › Word list: the language and the search.
    WordList,
    /// One word of one BIP-39 list.
    Word,
    /// Tools › Dice passphrase: the list, the length, the rolls and the
    /// passphrase.
    DicePassphrase,
    /// Tools › one calculator's field.
    Tool,
    /// Tools › one calculator's answer.
    ToolResult,
    /// Tools › Decode a transaction: the Sign review with no keys.
    Decode,
    /// Tools › Compare transactions: the second transaction's way in,
    /// and the result that says what differs.
    CompareTx,
    /// Tools › Notes: the notes of the session and the two ways to one.
    Notes,
    /// A note being typed.
    NoteText,
    /// A note, or an opened recovery sheet, shown whole.
    Note,
    /// One wallet's recovery sheet.
    Sheet,
    /// A silent payments wallet's address, or one of its labels
    /// (`docs/PLANNING.md` §16.113).
    SilentAddress,
    /// That wallet's labels, as addresses of their own.
    SilentLabels,
    /// Its check of one transaction.
    SilentCheck,
    /// The two names BIP-353's record is published under.
    SilentDns,
    /// BIP-392's scan descriptor, which carries the scan private key.
    SilentSecret,
    /// "Which form?": how an export leaves the device.
    ExportForm,
    /// The passphrase a note or a sheet is sealed under.
    SealPass,
    /// The sealed file, and the ways to take it off the device.
    Sealed,
    /// That file as one QR.
    SealedQr,
    /// The explanation of the assurance tiers.
    Tiers,
    /// The list of Learn pages.
    Learn,
    /// One Learn page.
    LearnPage,
    /// The hold-to-wipe screen.
    WipeAll,
    /// The Result after a wipe: what it removed.
    Wiped,
    /// The hold-to-wipe-and-exit screen.
    WipeAndExit,
    /// The terminal screen after wipe-and-exit: nothing is left and
    /// nothing can be done, until the shell closes.
    Ended,
    /// What a Tier A device says about secure boot, once at start.
    NoSecureBoot,
    /// The blocking self-test failure screen.
    SelfTestFailed,
    /// The blocking screen a device whose boot was not verified gets
    /// instead of the app.
    BootRefused,
    /// The lock screen: fingerprints and a PIN pad, nothing else.
    Lock,
}

impl ScreenKind {
    /// Every screen the application has. A test walks it to assert that
    /// each one is built from [`osk_ui::screens`] (`docs/DESIGN.md` §5:
    /// "Every flow is built from these sixteen screens").
    pub const ALL: [ScreenKind; 68] = [
        ScreenKind::Home,
        ScreenKind::StartHere,
        ScreenKind::Created,
        ScreenKind::Keys,
        ScreenKind::PickKey,
        ScreenKind::Wallets,
        ScreenKind::Add,
        ScreenKind::AddWallet,
        ScreenKind::WalletKeys,
        ScreenKind::KeyDetail,
        ScreenKind::Wallet,
        ScreenKind::WalletName,
        ScreenKind::Addresses,
        ScreenKind::BackupMenu,
        ScreenKind::OpenPassphrase,
        ScreenKind::OpenChild,
        ScreenKind::Bip85,
        ScreenKind::Opened,
        ScreenKind::Lightning,
        ScreenKind::Vanity,
        ScreenKind::Forget,
        ScreenKind::Keep,
        ScreenKind::StoredKey,
        ScreenKind::DuressPin,
        ScreenKind::KeptRemoved,
        ScreenKind::Load,
        ScreenKind::Build,
        ScreenKind::Create,
        ScreenKind::Backup,
        ScreenKind::Settings,
        ScreenKind::Setting,
        ScreenKind::About,
        ScreenKind::Sign,
        ScreenKind::Files,
        ScreenKind::Scan,
        ScreenKind::Verify,
        ScreenKind::SignMessage,
        ScreenKind::CheckedMessage,
        ScreenKind::Export,
        ScreenKind::Inspect,
        ScreenKind::Explore,
        ScreenKind::Tools,
        ScreenKind::WordList,
        ScreenKind::Word,
        ScreenKind::DicePassphrase,
        ScreenKind::Tool,
        ScreenKind::ToolResult,
        ScreenKind::Decode,
        ScreenKind::CompareTx,
        ScreenKind::Notes,
        ScreenKind::NoteText,
        ScreenKind::Note,
        ScreenKind::Sheet,
        ScreenKind::ExportForm,
        ScreenKind::SealPass,
        ScreenKind::Sealed,
        ScreenKind::SealedQr,
        ScreenKind::Tiers,
        ScreenKind::Learn,
        ScreenKind::LearnPage,
        ScreenKind::WipeAll,
        ScreenKind::Wiped,
        ScreenKind::WipeAndExit,
        ScreenKind::Ended,
        ScreenKind::NoSecureBoot,
        ScreenKind::SelfTestFailed,
        ScreenKind::BootRefused,
        ScreenKind::Lock,
    ];
}

/// Whose addresses a screen is showing: a registered policy, by its
/// index in `wallets`, or the words a key derives at a script type the
/// screen has chosen. Only a policy is a wallet (`docs/PLANNING.md`
/// §16.104); the other two are a key's own addresses, which Explore and
/// the first run's "Check an address" list without a wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalletRef {
    /// One loaded key, by its index in `keys`, at the script type the
    /// address list has chosen. Never a wallet: it has no page, no
    /// export and no row on Wallets.
    Key(usize),
    /// A wallet policy in use.
    Policy(usize),
    /// The words typed into Explore, which derive addresses the same
    /// way a loaded key does. Only the Addresses screen takes this:
    /// typed words are on no list and have no page of their own.
    Typed,
}

/// One public account of one key (`docs/PLANNING.md` §16.110 rule 1):
/// the four single-signature accounts BIP 44, 49, 84 and 86 define, and
/// the BIP-48 multisig accounts a coordinator asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Account {
    /// `m/purpose'/coin'/0'`, which derives addresses on its own.
    Single(ScriptType),
    /// `m/48'/coin'/0'/script'`, which derives none without the
    /// cosigners.
    Multi(MultisigScriptType),
}

impl Account {
    /// Every account the Choice offers, single-signature first, each
    /// list in its own crate's order.
    pub(crate) const ALL: [Account; 6] = [
        Account::Single(ScriptType::Legacy),
        Account::Single(ScriptType::NestedSegwit),
        Account::Single(ScriptType::NativeSegwit),
        Account::Single(ScriptType::Taproot),
        Account::Multi(MultisigScriptType::NestedSegwit),
        Account::Multi(MultisigScriptType::NativeSegwit),
    ];

    /// The account's name, which is the Choice's row.
    pub(crate) fn name(self, s: &Strings) -> &'static str {
        match self {
            Account::Single(ScriptType::Legacy) => s.account_legacy,
            Account::Single(ScriptType::NestedSegwit) => s.account_nested,
            Account::Single(ScriptType::NativeSegwit) => s.account_segwit,
            Account::Single(ScriptType::Taproot) => s.account_taproot,
            Account::Multi(MultisigScriptType::NestedSegwit) => s.account_multisig_nested,
            Account::Multi(MultisigScriptType::NativeSegwit) => s.account_multisig_segwit,
        }
    }

    /// The path under that name, for `network`, written with the `h`
    /// this app writes everywhere else.
    pub(crate) fn path(self, network: Network) -> String {
        let coin = network.coin_type();
        match self {
            Account::Single(script) => alloc::format!("m/{}h/{coin}h/0h", script.purpose()),
            Account::Multi(script) => alloc::format!("m/48h/{coin}h/0h/{}h", script.index()),
        }
    }
}

/// What an Export screen is exporting: a wallet, or one account of one
/// key (`docs/PLANNING.md` §16.110 rule 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exported {
    /// A wallet's own descriptor, policy or record.
    Wallet(WalletRef),
    /// One loaded key's account, by the key's index in `keys`.
    Account(usize, Account),
}

/// The account a key's Export is showing, derived once when the account
/// was chosen rather than on every frame.
enum AccountView {
    /// A single-signature account, which has a SLIP-132 form.
    Single(AccountXpub),
    /// A BIP-48 account, which has a BIP 129 key record.
    Multi(MultisigAccountXpub),
}

/// What the "Which key?" Choice is choosing a key for: the two ways a
/// key opens another key, asked only while a wallet's or a
/// transaction's row is waiting for one key in particular
/// (`docs/PLANNING.md` §16.104 rule 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickFor {
    /// Add a key › "Open with passphrase".
    Passphrase,
    /// Add a key › "Open BIP-85 child".
    Child,
}

/// Where a widget the next frame scrolls to lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Anchor {
    /// Fully inside the clip, and no further: the path editor's checked
    /// preset row (`docs/DESIGN.md` §4.6).
    InView,
    /// At the top of the view: the heading a Learn page opened at one of
    /// its sections starts on (`docs/PLANNING.md` §16.105).
    Top,
}

/// Which row opened a key derived from a loaded one, which is what the
/// Result after it is titled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenedFrom {
    /// Key › Open passphrase.
    Passphrase,
    /// Key › Open BIP-85 child.
    Child,
}

/// Where a key's account export is (`docs/PLANNING.md` §16.110 rule 2).
/// Every format but BIP 129's key record is [`KeyExportStep::Export`]
/// from the start; the record asks for its two values first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyExportStep {
    /// The Export screen: the format, the string, the QR and Copy.
    Export,
    /// The session token, on the hex keyboard.
    Token,
    /// The description, on the name keyboard.
    Description,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    /// Start here, opened from Settings. The first run's own copy is
    /// what Home draws while nothing is loaded (`start_here_shown`).
    StartHere,
    /// The Result after the first created key, by the key's index.
    Created(usize),
    /// The list of loaded keys.
    Keys,
    /// The list of registered wallets.
    Wallets,
    /// The menu of the four ways a key arrives.
    Add,
    /// The menu of the kinds of wallet.
    AddWallet,
    /// "Which key?", for what it is being chosen for.
    PickKey(PickFor),
    /// One wallet's members, as the Keys screen filtered to them.
    WalletKeys(usize),
    KeyDetail(usize),
    /// One key's account, for a coordinator: the Export screen, and the
    /// two Entry steps BIP 129's key record needs before it
    /// (`docs/PLANNING.md` §16.110 rules 1 and 2). The steps are steps
    /// of this screen, as the export's QR page is, so that the chevron
    /// walks back through them without the stack.
    KeyExport(usize, Account, KeyExportStep),
    /// The transaction's Keys review: the keys a transaction names, in
    /// a wallet's Keys review shape (§16.110 rule 3). `true` where the
    /// transaction is being read rather than signed.
    SignKeys(bool),
    /// One wallet's menu: a loaded key's single-sig wallet, or a policy
    /// in use.
    Wallet(WalletRef),
    /// That wallet's name, being typed.
    WalletName(WalletRef),
    Addresses(WalletRef),
    BackupMenu(usize),
    OpenPassphrase(usize),
    OpenChild(usize),
    /// One of BIP-85's other applications over the key at this index
    /// (§16.114). Which application, and where the flow is, are in
    /// [`Bip85Flow`] beside the screen, as the child flow's are.
    Bip85(usize),
    /// The vanity grinder over the key at this index (§16.117).
    /// Which dial, and where the flow is, are in [`vanity::Grind`]
    /// beside the screen, as the BIP-85 flow's are.
    Vanity(usize),
    /// The key that was opened, by its index in `keys`, and the row it
    /// was opened from.
    Opened(usize, OpenedFrom),
    Forget(usize),
    Keep(usize),
    DuressPin,
    KeptRemoved,
    Settings,
    About,
    Sign,
    Scan,
    Verify,
    SignMessage,
    CheckedMessage,
    Export(WalletRef),
    Inspect,
    Explore,
    Tools,
    /// Tools › Word list.
    WordList,
    /// One word of a BIP-39 list, by its 0-based index in the list the
    /// tool has open.
    Word(u16),
    /// Tools › Dice passphrase.
    DicePassphrase,
    /// Tools › one calculator, at its field.
    Tool(Tool),
    /// Tools › one calculator, at its answer.
    ToolResult(Tool),
    /// Tools › Decode a transaction.
    Decode,
    /// Tools › Compare transactions (`docs/PLANNING.md` §16.111 rule
    /// 4): the Menu that reads the second transaction while only the
    /// first has arrived, and the Result once both have.
    CompareTx,
    /// Tools › Lightning node key (`docs/PLANNING.md` §16.116): the
    /// passphrase of an LND cipher seed, the node's public key, and the
    /// Secret screen behind it. Which source, and what it answered, are
    /// in [`Lightning`] beside the screen.
    Lightning,
    /// Tools › Notes: the notes of the session and the two ways to one
    /// (`docs/PLANNING.md` §16.112 rule 2).
    Notes,
    /// A note being typed.
    NoteText,
    /// The note in hand, whole.
    Note,
    /// One wallet's recovery sheet.
    Sheet(WalletRef),
    /// A silent payments wallet's address: the wallet's index, and the
    /// label it is the address of, `None` for the address itself
    /// (`docs/PLANNING.md` §16.113).
    SilentAddress(usize, Option<u32>),
    /// That wallet's labels.
    SilentLabels(usize),
    /// Its check of one transaction.
    SilentCheck(usize),
    /// The two names BIP-353's record is published under, for the
    /// wallet whose export asked for them.
    SilentDns(usize),
    /// BIP-392's scan descriptor for that wallet, on a Secret screen.
    SilentSecret(usize),
    /// A recovery sheet that arrived encrypted, opened.
    OpenedSheet,
    /// "Which form?": how what is being exported leaves the device.
    ExportForm,
    /// The passphrase a note or a sheet is sealed under, typed twice.
    SealPass,
    /// The file that sealing made, with the ways to take it off the
    /// device.
    Sealed,
    /// That file as one QR.
    SealedQr,
    Tiers,
    Learn,
    /// One Learn page, by its index in the list.
    LearnPage(usize),
    /// One Learn page, opened from a working screen's info button
    /// (`docs/PLANNING.md` §16.105): its index in the list, and the
    /// section it starts at. It is drawn over whatever is on the screen,
    /// a wizard included, and the chevron puts that screen back with
    /// everything beside the stack untouched. The page draws no "Try it"
    /// row: the person is already in the flow.
    LearnTopic(usize, Option<usize>),
    WipeAll,
    /// A finished wipe and how many keys it removed.
    Wiped(usize),
    WipeAndExit,
    Ended,
    /// What a Tier A device opens on, before Home.
    NoSecureBoot,
}

impl Screen {
    fn kind(self) -> ScreenKind {
        match self {
            Screen::Home => ScreenKind::Home,
            Screen::StartHere => ScreenKind::StartHere,
            Screen::Created(_) => ScreenKind::Created,
            Screen::Keys => ScreenKind::Keys,
            Screen::Wallets => ScreenKind::Wallets,
            Screen::Add => ScreenKind::Add,
            Screen::AddWallet => ScreenKind::AddWallet,
            Screen::PickKey(_) => ScreenKind::PickKey,
            Screen::WalletKeys(_) => ScreenKind::WalletKeys,
            Screen::KeyDetail(_) => ScreenKind::KeyDetail,
            // The token and the description are steps of the export, as
            // its QR page is, so the three carry one kind.
            Screen::KeyExport(..) => ScreenKind::Export,
            // The same screen as a wallet's Keys review, over the keys a
            // transaction names rather than a wallet's members.
            Screen::SignKeys(_) => ScreenKind::WalletKeys,
            Screen::Wallet(_) => ScreenKind::Wallet,
            Screen::Notes => ScreenKind::Notes,
            Screen::NoteText => ScreenKind::NoteText,
            Screen::Note | Screen::OpenedSheet => ScreenKind::Note,
            Screen::Sheet(_) => ScreenKind::Sheet,
            Screen::SilentAddress(..) => ScreenKind::SilentAddress,
            Screen::SilentLabels(_) => ScreenKind::SilentLabels,
            Screen::SilentCheck(_) => ScreenKind::SilentCheck,
            Screen::SilentDns(_) => ScreenKind::SilentDns,
            Screen::SilentSecret(_) => ScreenKind::SilentSecret,
            Screen::ExportForm => ScreenKind::ExportForm,
            Screen::SealPass => ScreenKind::SealPass,
            Screen::Sealed => ScreenKind::Sealed,
            Screen::SealedQr => ScreenKind::SealedQr,
            Screen::WalletName(_) => ScreenKind::WalletName,
            Screen::Addresses(_) => ScreenKind::Addresses,
            Screen::BackupMenu(_) => ScreenKind::BackupMenu,
            Screen::OpenPassphrase(_) => ScreenKind::OpenPassphrase,
            Screen::OpenChild(_) => ScreenKind::OpenChild,
            Screen::Bip85(_) => ScreenKind::Bip85,
            Screen::Vanity(_) => ScreenKind::Vanity,
            Screen::Opened(..) => ScreenKind::Opened,
            Screen::Forget(_) => ScreenKind::Forget,
            Screen::Keep(_) => ScreenKind::Keep,
            Screen::DuressPin => ScreenKind::DuressPin,
            Screen::KeptRemoved => ScreenKind::KeptRemoved,
            Screen::Settings => ScreenKind::Settings,
            Screen::About => ScreenKind::About,
            Screen::Sign => ScreenKind::Sign,
            Screen::Scan => ScreenKind::Scan,
            Screen::Verify => ScreenKind::Verify,
            Screen::SignMessage => ScreenKind::SignMessage,
            Screen::CheckedMessage => ScreenKind::CheckedMessage,
            Screen::Export(_) => ScreenKind::Export,
            Screen::Inspect => ScreenKind::Inspect,
            Screen::Explore => ScreenKind::Explore,
            Screen::Tools => ScreenKind::Tools,
            Screen::WordList => ScreenKind::WordList,
            Screen::Word(_) => ScreenKind::Word,
            Screen::DicePassphrase => ScreenKind::DicePassphrase,
            Screen::Tool(_) => ScreenKind::Tool,
            Screen::ToolResult(_) => ScreenKind::ToolResult,
            Screen::Decode => ScreenKind::Decode,
            Screen::CompareTx => ScreenKind::CompareTx,
            Screen::Lightning => ScreenKind::Lightning,
            Screen::Tiers => ScreenKind::Tiers,
            Screen::Learn => ScreenKind::Learn,
            Screen::LearnPage(_) | Screen::LearnTopic(..) => ScreenKind::LearnPage,
            Screen::WipeAll => ScreenKind::WipeAll,
            Screen::Wiped(_) => ScreenKind::Wiped,
            Screen::WipeAndExit => ScreenKind::WipeAndExit,
            Screen::Ended => ScreenKind::Ended,
            Screen::NoSecureBoot => ScreenKind::NoSecureBoot,
        }
    }
}

/// A wizard or flow drawn over the current screen. Dropping it zeroizes
/// everything it held. The variants differ in size because each carries
/// its secrets inline (`docs/PLANNING.md` §5.3: no heap allocation of
/// plaintext secrets), so they are not boxed.
#[allow(clippy::large_enum_variant)]
enum Wizard {
    Load(LoadWizard),
    Build(WalletWizard),
    Create(CreateWizard),
    Backup(BackupFlow),
}

/// The word counts "Open BIP-85 child seed" offers. BIP-85's
/// application 39' defines five; these are the three in common use.
pub const CHILD_COUNTS: [usize; 3] = [12, 18, 24];

/// The characters BIP 129 allows a key record's description
/// (`docs/PLANNING.md` §16.110 rule 2).
pub(crate) const BSMS_DESCRIPTION_MAX: usize = 80;

/// How far past that the field takes characters, so that the refusal is
/// something a person can read and correct rather than a key that does
/// nothing.
const BSMS_DESCRIPTION_TYPED: usize = 96;

/// `[73c5da0a/48h/0h/0h/2h]xpub…`: an account key with the origin every
/// coordinator asks for, written with the `h` this app writes
/// everywhere else (§2.1).
pub(crate) fn origin_key(fingerprint: Fingerprint, path: &DerivationPath, xpub: &Xpub) -> String {
    let mut text = String::new();
    // Formatting into a String cannot fail.
    let _ = write!(text, "[{fingerprint}");
    for child in path {
        let _ = write!(text, "/{child:#}");
    }
    let _ = write!(text, "]{xpub}");
    text
}

/// The largest BIP-85 index, which is the largest hardened child.
const MAX_CHILD_INDEX: u32 = osk_bip::bip85::MAX_INDEX;

/// Digits the index field takes, which is what [`MAX_CHILD_INDEX`] is
/// written in.
const MAX_CHILD_INDEX_CHARS: usize = 10;

/// Where "Open BIP-85 child seed" is: the word count, then the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildStep {
    /// Choosing how many words the child has.
    Words,
    /// Typing the child's index.
    Index,
}

/// The word count and index being chosen for a BIP-85 child. Neither is
/// a secret: they are the two numbers of a path.
struct ChildFlow {
    step: ChildStep,
    words: usize,
    index: String,
    /// The fingerprint of the child the typed index derives, recomputed
    /// on every keystroke and `None` while the index is empty or out of
    /// range (`docs/PLANNING.md` §16.67). A fingerprint is public data.
    fingerprint: Option<Fingerprint>,
}

impl ChildFlow {
    fn new() -> Self {
        ChildFlow {
            step: ChildStep::Words,
            words: CHILD_COUNTS[0],
            index: String::new(),
            fingerprint: None,
        }
    }

    /// The index typed so far, when it is one BIP-85 can derive.
    fn index_value(&self) -> Option<u32> {
        self.index
            .parse::<u64>()
            .ok()
            .filter(|n| *n <= u64::from(MAX_CHILD_INDEX))
            .map(|n| n as u32)
    }

    /// Whether what is typed is out of range, which the caption says.
    fn index_error(&self) -> bool {
        !self.index.is_empty() && self.index_value().is_none()
    }

    /// One step back, or `false` to leave the screen.
    fn back(&mut self) -> bool {
        match self.step {
            ChildStep::Words => false,
            ChildStep::Index => {
                self.index.clear();
                self.fingerprint = None;
                self.step = ChildStep::Words;
                true
            }
        }
    }
}

/// The byte counts "How many bytes?" offers for BIP-85's hex
/// application. The BIP takes 16 to 64; these are the three lengths a
/// person has a use for (`docs/PLANNING.md` §16.114).
pub const HEX_BYTE_COUNTS: [usize; 3] = [16, 32, 64];

/// Which BIP-85 application the key page's row is deriving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bip85App {
    /// Application 39': a child key, which goes to the child flow.
    Words,
    /// Application 2': an HD-seed WIF.
    Wif,
    /// Application 32': an extended private key.
    Xprv,
    /// Application 128169': raw bytes as hex.
    Hex,
    /// Application 707764': a base64 password.
    Base64,
    /// Application 707785': a base85 password.
    Base85,
}

impl Bip85App {
    /// Every application, in the order "Which application?" lists them.
    pub const ALL: [Bip85App; 6] = [
        Bip85App::Words,
        Bip85App::Wif,
        Bip85App::Xprv,
        Bip85App::Hex,
        Bip85App::Base64,
        Bip85App::Base85,
    ];

    /// The length bounds of a password application, `None` for the
    /// applications that have no length to ask for.
    fn lengths(self) -> Option<(usize, usize)> {
        match self {
            Bip85App::Base64 => Some(osk_bip::bip85::BASE64_LENGTHS),
            Bip85App::Base85 => Some(osk_bip::bip85::BASE85_LENGTHS),
            _ => None,
        }
    }

    /// The length the field starts with, which is the one BIP-85's own
    /// vector uses: 21 characters of base64, 12 of base85.
    fn default_length(self) -> usize {
        match self {
            Bip85App::Base64 => 21,
            _ => 12,
        }
    }
}

/// Where the BIP-85 row is: the length of a password, the index, and
/// the derived value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bip85Step {
    /// A password's length, on the digits pad.
    Length,
    /// The index, on the digits pad.
    Index,
    /// The value, on a §5 Secret screen.
    Value,
}

/// What is being derived and what has been typed for it. None of it is
/// a secret: an application, two numbers and a byte count are the steps
/// of a path. The value itself is derived where it is drawn and kept
/// nowhere (§16.114).
struct Bip85Flow {
    app: Bip85App,
    step: Bip85Step,
    /// How many bytes the hex application asks for.
    bytes: usize,
    /// The password length typed so far.
    length: String,
    /// The index typed so far.
    index: String,
}

impl Bip85Flow {
    fn new(app: Bip85App) -> Self {
        Bip85Flow {
            app,
            step: if app.lengths().is_some() {
                Bip85Step::Length
            } else {
                Bip85Step::Index
            },
            bytes: HEX_BYTE_COUNTS[0],
            length: alloc::format!("{}", app.default_length()),
            index: String::from("0"),
        }
    }

    /// The length typed so far, when it is one the application takes.
    fn length_value(&self) -> Option<usize> {
        let (low, high) = self.app.lengths()?;
        self.length
            .parse::<usize>()
            .ok()
            .filter(|n| (low..=high).contains(n))
    }

    /// The index typed so far, when it is one BIP-85 can derive.
    fn index_value(&self) -> Option<u32> {
        self.index
            .parse::<u64>()
            .ok()
            .filter(|n| *n <= u64::from(MAX_CHILD_INDEX))
            .map(|n| n as u32)
    }

    /// One step back, or `false` to leave the screen.
    fn back(&mut self) -> bool {
        match self.step {
            Bip85Step::Length => false,
            Bip85Step::Index => {
                if self.app.lengths().is_none() {
                    return false;
                }
                self.step = Bip85Step::Length;
                true
            }
            Bip85Step::Value => {
                self.step = Bip85Step::Index;
                true
            }
        }
    }
}

/// What the quiz screen shows, for tests (`docs/PLANNING.md` §16.7): the
/// same text as the pixels, nothing more.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuizView {
    /// 1-based position of the word being asked.
    pub word_number: usize,
    /// The four candidates as labelled, in button order.
    pub choices: Vec<String>,
    /// Words answered correctly so far.
    pub done: usize,
    /// Words in the quiz.
    pub total: usize,
    /// Where the quiz is.
    pub state: QuizState,
    /// Whether the helper variant is on.
    pub helper: bool,
    /// The wrong-answer card's text, while one shows.
    pub message: Option<String>,
}

/// The launcher's six tiles, in grid order (`docs/PLANNING.md` §16.104
/// rule 4). Indices are [`ids::HOME_TILE_BASE`] offsets, and the `wide`
/// sidebar lists the same six in the same order.
pub const HUB_TILES: [Icon; 6] = [
    Icon::Wallet,
    Icon::Keys,
    Icon::Scan,
    Icon::Tools,
    Icon::Learn,
    Icon::Settings,
];
const TILE_WALLETS: usize = 0;
const TILE_KEYS: usize = 1;
const TILE_SCAN: usize = 2;
const TILE_TOOLS: usize = 3;
const TILE_LEARN: usize = 4;
const TILE_SETTINGS: usize = 5;

/// The label of tile `i`.
pub(crate) fn hub_label(i: usize, s: &Strings) -> &'static str {
    match i {
        TILE_WALLETS => s.home_wallets,
        TILE_KEYS => s.home_keys,
        TILE_SCAN => s.home_scan,
        TILE_TOOLS => s.home_tools,
        TILE_LEARN => s.home_learn,
        _ => s.home_settings,
    }
}

/// The tiles as the launcher draws them.
pub(crate) fn hub_tiles(s: &Strings) -> Vec<osk_ui::components::Tile> {
    HUB_TILES
        .iter()
        .enumerate()
        .map(|(i, icon)| osk_ui::components::Tile {
            id: ids::at(ids::HOME_TILE_BASE, i),
            icon: *icon,
            label: String::from(hub_label(i, s)),
            enabled: true,
            badge: None,
        })
        .collect()
}

/// Highest address index the explorer walks to.
pub const MAX_ADDRESS_INDEX: u32 = 199;

/// How long a second Back leaves the app after a first one pressed where
/// nothing is behind, in milliseconds. Long enough to read the line that
/// says so and press again, short enough that a Back pressed by mistake
/// is forgotten before the next one.
pub const LEAVE_WINDOW_MS: u64 = 2_000;

/// How long a screen states what just happened to the clipboard —
/// "Copied", "No clipboard", "Nothing to paste" — in milliseconds. Long
/// enough to read, short enough that the screen is itself again by the
/// time the next thing is done on it.
pub const NOTICE_MS: u64 = 2_000;

/// Words on one backup page of a `Small` panel (UX.md D1). A secret
/// panel never pages, so this is also what fits one panel; a taller
/// screen holds twelve.
pub const PAGE_WORDS: usize = views::words::PAGE_WORDS;

/// How long the app bar's eye shows a transcription panel, in
/// milliseconds. Long enough to write a page of words down with both
/// hands, short enough that a screen left face up masks itself.
pub const REVEAL_MS: u64 = 30_000;

/// Addresses the list shows at first, and how many more the "More" row
/// adds each time (`docs/DESIGN.md` §5 Addresses: a list, not a pager).
pub const ADDRESS_PAGE: usize = 10;

/// The address list's state: script type, chain, how far the list has
/// been extended, and the one address whose own screen is open.
struct DetailState {
    script: ScriptType,
    change: bool,
    /// How many rows the list holds; "More" adds [`ADDRESS_PAGE`].
    shown: usize,
    /// The address the §5 Address screen is showing, by index.
    open: Option<u32>,
}

impl Default for DetailState {
    fn default() -> Self {
        DetailState {
            script: ScriptType::NativeSegwit,
            change: false,
            shown: ADDRESS_PAGE,
            open: None,
        }
    }
}

/// The formats the wallet export offers, one per screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// The output descriptor, with key origin and checksum.
    Descriptor,
    /// The account extended public key.
    Xpub,
    /// The SLIP-132 form of the account key.
    Slip132,
    /// A wallet's BIP-388 policy: the template and its keys.
    Policy,
    /// BIP 129's descriptor record: round 2 of a Bitcoin Secure Multisig
    /// Setup, which is the wallet with its paths and its first address.
    Bsms,
    /// BIP 129's key record: round 1, which is one key's multisig
    /// account signed by that account's own key (§16.110 rule 2).
    BsmsSigner,
    /// Bitcoin Core's `importdescriptors` file: the wallet's multipath
    /// descriptor, active, over the first thousand addresses, from the
    /// rescan point the Choice asked for (§16.114).
    CoreImport,
    /// BIP-392's `sp(spscan1q…)`: the scan private key and the spend
    /// public key, which is what a scanner needs and is a secret
    /// (§16.113).
    SilentScan,
    /// BIP-321's `bitcoin:?sp=…` for a silent payments wallet.
    SilentUri,
    /// BIP-353's TXT record line for that URI, under the user name and
    /// domain the two Entry steps asked for.
    SilentDns,
}

impl ExportFormat {
    /// Every format, in page order. A key is offered the first three and
    /// a wallet the descriptor and the policy; `format_offered` says
    /// which.
    pub const ALL: [ExportFormat; 10] = [
        ExportFormat::Descriptor,
        ExportFormat::Xpub,
        ExportFormat::Slip132,
        ExportFormat::Policy,
        ExportFormat::Bsms,
        ExportFormat::BsmsSigner,
        ExportFormat::CoreImport,
        ExportFormat::SilentScan,
        ExportFormat::SilentUri,
        ExportFormat::SilentDns,
    ];

    /// Where the account key sits in that list, which is the format an
    /// export falls back to when the one it had is no longer offered.
    pub(crate) fn xpub_index() -> usize {
        ExportFormat::ALL
            .iter()
            .position(|f| *f == ExportFormat::Xpub)
            .expect("the account key is always a format")
    }
}

/// Export screen state: the account, the format, whether the QR screen
/// the "Show as QR" row opens is over the menu, and — because §4.9 says
/// "The wallet export animates on the panel like any other payload" —
/// the animated run it shows when one code would fall below the module
/// pitch floor.
struct ExportState {
    script: ScriptType,
    format: usize,
    qr: bool,
    /// Whether the QR is animated. Forced on where one code would be
    /// below the floor (§4.9).
    animated: bool,
    /// Whether one code clears the floor at the class's side on this
    /// display; `false` dims the toggle with "too dense".
    scans: bool,
    /// The largest version the class's square keeps above the floor,
    /// which is what the run's part count follows from.
    version: u8,
    /// The animated run, while one is on screen.
    run: Option<codes::UrRun>,
}

impl Default for ExportState {
    fn default() -> Self {
        ExportState {
            script: ScriptType::NativeSegwit,
            format: 0,
            qr: false,
            animated: false,
            scans: true,
            version: 1,
            run: None,
        }
    }
}

/// The two values BIP 129's key record is written from, and the record
/// once it is written (`docs/PLANNING.md` §16.110 rule 2).
///
/// All three are public: a session token is a nonce the coordinator
/// chose, a description is a label, and the record is what leaves the
/// device.
#[derive(Debug, Default)]
struct BsmsEntry {
    /// The session token, hex, being typed.
    token: String,
    /// The description, being typed.
    description: String,
    /// The record, once the description was accepted.
    record: Option<String>,
}

/// A row the current screen draws dimmed (`docs/DESIGN.md` §4.11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimmedRow {
    /// What the row would open.
    pub label: String,
    /// Why it cannot, in two or three words; `None` on a row whose only
    /// answer is that the feature is not built.
    pub reason: Option<String>,
    /// Whether the reason is the row's second line, which is where
    /// §4.11 puts it on every class.
    pub below: bool,
}

/// The screens a row opens over the one it is on (`docs/DESIGN.md` §4.2
/// and §4.5): the Choice behind a value row, the Compare behind a
/// reference row. The screen under it keeps its state, because reading
/// an address whole is not leaving the transaction.
pub(crate) enum Overlay {
    /// A Choice screen: which list, and the row the check is on.
    Choice(Picker, usize),
    /// §4.2 Setting: the same list with the check on the current value
    /// and no Continue. The tap applies and the screen stays.
    Setting(Setting),
    /// A Compare screen: what the string is and the string itself.
    Compare(Comparison),
    /// §5 Menu, "Files": what a shell that can list its files answered a
    /// file request with. It sits over the screen that asked, so the
    /// file the person taps lands where the request came from.
    Files(FileListing),
}

/// §4.3: a tap on a candidate cell. A strip numbers a cell within
/// itself; the second row's cells follow the first row's in the candidate
/// list, and a Chinese list's rows are ten cells wide (§16.45). The last
/// cell pages when more candidates remain than the strip holds — twenty
/// cells hold every tone group of the readings as they stand, so that is
/// the fallback for a group of twenty-one or more.
fn candidate_tap(w: &mut LoadWizard, class: SizeClass, second_row: bool, n: u8) {
    let one_char = w.language().max_display_chars() == 1;
    let index =
        usize::from(n) + usize::from(second_row) * tokens::candidates_per_row(class, one_char);
    let shown = tokens::candidates(class, one_char);
    if index + 1 == shown && w.more_candidates(shown) {
        w.page_forward(shown - 1);
    } else {
        w.commit_candidate(index);
    }
}

/// The files one shell listed for one request (`docs/DESIGN.md` §5
/// Menu, "Files"), in the order the shell gave them, newest first.
pub(crate) struct FileListing {
    /// The kind that was requested, which the read carries back.
    pub kind: FileKind,
    /// The files, newest first.
    pub entries: Vec<FileEntry>,
    /// What the shell called the place it looked, for an empty list.
    pub place: Option<String>,
}

/// The settings whose value row opens a Choice in §4.2's Setting form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// The chain every key is derived on.
    Network,
    /// The unit every amount is shown in (§4.7).
    Unit,
    /// The auto-lock timer.
    Lock,
    /// The auto-wipe timer.
    Wipe,
    /// How far a camera frame is turned before it is read (§4.9).
    CameraRotation,
    /// Which RFC 6979 nonce every ECDSA signature uses (§16.38).
    Nonce,
    /// Where a Schnorr signature's auxiliary randomness comes from.
    Schnorr,
    /// The Argon2id memory an encrypted export is made at (§16.112).
    BackupMemory,
}

/// The flow the last row of a Learn page opens (UX.md §7.1): the page
/// says what the thing is, and the row is where it is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TryIt {
    /// The Create wizard.
    CreateKey,
    /// The Create wizard past its source step, with dice chosen.
    RollDice,
    /// The first loaded key's backup menu.
    VerifyBackup,
    /// That key's encrypted backup.
    EncryptedBackup,
    /// That key's "Open passphrase".
    OpenPassphrase,
    /// The first wallet's "Check an address".
    CheckAddress,
    /// The wallet builder.
    NewWallet,
    /// Tools › Miniscript.
    Miniscript,
    /// The first wallet's export.
    Export,
    /// The first single-sig wallet's "Sign a message".
    SignMessage,
}

/// The units §4.7 offers, in the order the Choice lists them.
pub const UNITS: [Unit; 2] = [Unit::Sat, Unit::Btc];

/// The nonces the setting offers, in the order the Choice lists them.
pub const NONCES: [osk_psbt::Nonce; 2] = [osk_psbt::Nonce::LowR, osk_psbt::Nonce::First];

/// The Schnorr auxiliary-randomness choices, in the order the Choice
/// lists them. The deterministic one is first and is the default, so a
/// device compares byte for byte against another implementation unless
/// its owner asks for randomness instead.
pub const SCHNORRS: [osk_psbt::Schnorr; 2] =
    [osk_psbt::Schnorr::Deterministic, osk_psbt::Schnorr::Fresh];

/// The Argon2id memory costs an encrypted backup is offered at, in
/// KiB, in the order the Choice lists them (`docs/PLANNING.md`
/// §16.112). More memory makes every guess cost more and every open
/// slower, and a file made at a cost a device cannot allocate cannot be
/// opened there. Passes stay three and lanes one, as they are
/// everywhere else in the app.
pub const BACKUP_MEMORY: [u32; 3] = [65_536, 262_144, 1_048_576];

/// The memory a device needs before 256 MiB is the recommendation, in
/// MiB. Below it the recommendation is the first row, which opens
/// anywhere.
pub const BACKUP_MEMORY_RECOMMENDS_MORE: u32 = 1024;

/// The cost of `memory_kib`, with the passes and lanes every Argon2id
/// in this app uses.
pub(crate) fn backup_cost(memory_kib: u32) -> osk_backup::Cost {
    osk_backup::Cost {
        memory_kib,
        passes: osk_backup::DEVICE_PARAMS.passes,
        lanes: osk_backup::DEVICE_PARAMS.lanes,
    }
}

/// The forms an export leaves the device in, in the order "Which
/// form?" lists them (`docs/PLANNING.md` §16.112, §16.134). Every
/// export that can be encrypted asks this before the passphrase. A QR
/// is not a form: the Result shows the file as one where it fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// An `osk-backup` file, which the Choice calls "Encrypted backup".
    Oskb,
    /// A KDBX 4 database, which an heir opens in a KeePass app
    /// (`osk_backup::kdbx`).
    Kdbx,
    /// A plain text file, offered for a note and a sheet and for
    /// nothing that holds a key.
    Plain,
}

impl Form {
    /// The three, in the order the Choice lists them.
    pub const ALL: [Form; 3] = [Form::Oskb, Form::Kdbx, Form::Plain];
}

/// What "Which form?" is choosing a form for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormFor {
    /// One loaded key's encrypted backup, by the key's index: its
    /// words, or its master seed where it has no words.
    Backup(usize),
    /// The note on screen.
    Note,
    /// One wallet's recovery sheet.
    Sheet(WalletRef),
}

/// Which list a Choice overlay offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Picker {
    /// The four script types (§4.6).
    Script,
    /// The same four, for the wallet an extended public key on its own
    /// makes: the key says nothing about what it pays to.
    XpubScript,
    /// The three wallet-export formats.
    Format,
    /// The loaded keys, when more than one could sign a message.
    MessageKey(usize),
    /// The two message signature formats.
    MessageFormat,
    /// The four notations the word list is searched by.
    SearchBy,
    /// The three ways the Hashes field is read.
    ReadAs,
    /// The four units the Units field can be typed in.
    FromUnit,
    /// The two wrappers a compiled policy can be put in.
    PolicyScript,
    /// The loaded keys, for Convert key's "Use a loaded key" row.
    ConvertKey(usize),
    /// "Keep this wallet on the device?", after a passphrase wallet is
    /// added on a Tier B device that keeps keys (§16.104 rule 7).
    KeepWallet,
    /// "Which account?", over a key's page (§16.110 rule 1).
    Account,
    /// "Which application?", over a key's page (§16.114).
    Bip85App,
    /// "How many bytes?", for BIP-85's hex application.
    Bip85Bytes,
    /// "Rescan from?", before a wallet's Bitcoin Core import file.
    Rescan,
    /// "Which form?", over a silent payment address (§16.113).
    SilentForm,
    /// "From?", over Tools › Lightning node key (§16.116).
    LightningFrom,
    /// "How?", over a key's "Vanity address" row (§16.117).
    VanityDial,
    /// The loaded keys, for that tool's second row.
    LightningKey(usize),
}

/// Where a key added from a wallet's not-loaded member row comes back
/// to, and what that row named (`docs/PLANNING.md` §16.104 rule 6).
pub(crate) struct AddKeyReturn {
    /// The review the person was on.
    to: ReturnTo,
    /// The member the row stands for, where the row named one: what a
    /// key opened with a passphrase or as a BIP-85 child is compared
    /// with.
    member: Option<Member>,
}

/// Which Keys review a key added from a not-loaded row comes back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReturnTo {
    /// A wallet's, by the wallet's index (§16.104 rule 6).
    Wallet(usize),
    /// A transaction's (§16.110 rule 3).
    Transaction,
}

/// The transaction a key was added from the Keys review of, kept beside
/// the screen stack while Add a key runs.
///
/// Every screen change clears the Sign flow, which is what keeps a PSBT
/// from outliving its screen; a detour to Add a key would throw the
/// transaction away, so the flow is taken out whole here and put back
/// when the key arrives, re-inspected with the keys there are now.
struct KeyDetour {
    /// The flow as it stood.
    flow: SignFlow,
    /// Whether it was the reading flow rather than the signing one.
    reading: bool,
}

/// What a wallet's Keys review names one member by: a policy's key by
/// its master fingerprint, a FROST group's member by its public share
/// (§16.104 rule 3, "the two fingerprints of a member").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Member {
    /// A policy's key.
    Master(Fingerprint),
    /// A group's member.
    Share(PublicKey),
}

impl Member {
    /// Whether `key` is this member.
    fn matches(self, key: &LoadedKey) -> bool {
        match self {
            Member::Master(fp) => key.fingerprint == fp,
            Member::Share(p) => key.share == Some(p),
        }
    }

    /// The fingerprint the review names this member by.
    fn fingerprint(self) -> Option<Fingerprint> {
        match self {
            Member::Master(fp) => Some(fp),
            Member::Share(p) => Some(osk_bip::threshold::share_fingerprint(&p)),
        }
    }

    /// The fingerprint `key` carries where this member is named, which
    /// is what a mismatch states beside the one the wallet asked for.
    fn fingerprint_of(self, key: &LoadedKey) -> Option<Fingerprint> {
        match self {
            Member::Master(_) => Some(key.fingerprint),
            Member::Share(_) => key.share_fingerprint,
        }
    }
}

impl Picker {
    /// How many rows the list has.
    pub(crate) fn len(self) -> usize {
        match self {
            Picker::Script | Picker::XpubScript => ScriptType::ALL.len(),
            Picker::Format => ExportFormat::ALL.len(),
            Picker::MessageKey(keys) => keys,
            Picker::MessageFormat => message::FORMATS.len(),
            // The four notations, and the row that browses from word 1.
            Picker::SearchBy => wordlist::SearchBy::ALL.len() + 1,
            Picker::ReadAs => osk_codec::encodings::ReadAs::ALL.len(),
            Picker::FromUnit => osk_ui::components::Denomination::ALL.len(),
            Picker::PolicyScript => osk_bip::compile::PolicyScript::ALL.len(),
            Picker::ConvertKey(keys) => keys,
            // No and Yes.
            Picker::KeepWallet => 2,
            Picker::Account => Account::ALL.len(),
            Picker::Bip85App => Bip85App::ALL.len(),
            Picker::Bip85Bytes => HEX_BYTE_COUNTS.len(),
            // "The start" and "Now".
            Picker::Rescan => 2,
            Picker::SilentForm => silent::AddressForm::ALL.len(),
            // "An LND cipher seed" and "A loaded key".
            Picker::LightningFrom => 2,
            Picker::VanityDial => vanity::Dial::ALL.len(),
            Picker::LightningKey(keys) => keys,
        }
    }
}

/// One string on the Compare screen (§4.5): the label over it, the whole
/// string, and whether it is a descriptor, which is shown as structure
/// rather than chunked in fours.
pub(crate) struct Comparison {
    /// §5 Compare: the title names the kind — "Signature", "Address",
    /// "Descriptor", "Account key".
    pub(crate) title: String,
    /// The line over the string, which names the instance where there is
    /// one: "#0 · 73c5da0a", "Output 1". `None` where the kind is the
    /// whole of what there is to say, so the screen never says the same
    /// text twice.
    pub(crate) label: Option<String>,
    pub(crate) value: String,
    pub(crate) descriptor: bool,
}

/// One level of the explorer's path: public data only.
struct ExploreLevel {
    xpub: String,
}

/// What the Explore path and encodings sections show, cached per (key,
/// network, path) because the tree is rebuilt every frame. Public data
/// only: extended public keys, fingerprints and addresses. Private keys,
/// the seed and the words are derived while their hold button is held
/// and never cached.
struct ExploreCache {
    fingerprint: Fingerprint,
    network: Network,
    path: DerivationPath,
    master_xpub: String,
    levels: Vec<ExploreLevel>,
    /// The leaf's addresses: the implied script type, or all four.
    addresses: Vec<(ScriptType, String)>,
    /// The account-level key (the first three levels, or the leaf of a
    /// shorter path): the xpub, and the SLIP-132 form when the purpose
    /// has one.
    account_xpub: String,
    account_slip132: Option<(ScriptType, String)>,
}

impl ExploreCache {
    /// Whether the account key is a key of its own. §4.5 and §2.11: at a
    /// path no deeper than the account, `account_xpub` is the leaf, so an
    /// "Account key" row would repeat the "Extended public key" row's
    /// value with nothing on the screen to say why.
    fn has_account_key(&self) -> bool {
        self.path.as_ref().len() > explore::ACCOUNT_LEVELS
    }
}

/// The addresses of the wallet whose Addresses screen is open, derived
/// once rather than every frame. Public data only, and dropped whenever
/// another screen is entered.
struct WalletAddresses {
    wallet: usize,
    receive: Vec<String>,
    change: Vec<String>,
}

/// A derived account, cached because EC derivation is not free and the
/// tree is rebuilt every frame. Public data only.
struct CachedAccount {
    fingerprint: Fingerprint,
    script: ScriptType,
    account: AccountXpub,
    receive: Vec<String>,
    change: Vec<String>,
}

/// The application.
pub struct OpenSigner {
    tier: AssuranceTier,
    build: BuildInfo,
    canvas: Option<Canvas>,
    ctx: Option<LayoutCtx>,
    /// The display's physical density, which decides how wide a QR
    /// module is in millimetres (UX review 2026-09-07, §2.8).
    dpi: u16,
    ui: UiState,
    layout: Option<Layout>,
    /// The tree the last [`Self::render`] solved [`Self::layout`] from,
    /// kept so that the frame is painted when it is read rather than
    /// when the event landed (`docs/PLANNING.md` §16.119 rule 5).
    tree: Option<Node>,
    /// Whether an event has solved a layout that the canvas has not been
    /// painted from yet.
    dirty: bool,
    theme: Theme,
    commands: VecDeque<Command>,
    now_ms: u64,
    keys: Vec<LoadedKey>,
    network: Network,
    /// The unit every amount is shown in (§4.7, §2.9: one unit, never
    /// both).
    unit: Unit,
    /// How far every camera frame is turned before it is read (§4.9).
    camera_rotation: CameraRotation,
    /// Which RFC 6979 nonce every ECDSA signature uses (§16.38).
    nonce: osk_psbt::Nonce,
    /// Where a Schnorr signature's auxiliary randomness comes from
    /// (security review 2026-09-11, L3).
    schnorr: osk_psbt::Schnorr,
    /// How many Schnorr signings this session has drawn auxiliary
    /// randomness for, so no two draw the same.
    schnorr_draws: u32,
    /// The open MuSig2 signing session, while this device holds a secret
    /// nonce for a transaction that has not come back (§16.100). It ends
    /// at lock, wipe, exit, a session for another transaction, and the
    /// signing it was opened for.
    musig_session: Option<osk_psbt::MusigSession>,
    /// How many MuSig2 sessions this boot has drawn a seed for, so no
    /// two sessions draw the same.
    musig_draws: u32,
    /// Whether the shell's frames are always upright, from
    /// [`DisplayInfo::camera_fixed`]. When they are, the camera
    /// rotation is not offered and not applied.
    camera_fixed: bool,
    /// How much memory the shell says this device has, in MiB, or
    /// `None` where it could not say. It decides which Argon2id memory
    /// cost Settings recommends (`docs/PLANNING.md` §16.112).
    memory_mib: Option<u32>,
    /// The Argon2id memory an encrypted export is made at, in KiB, once
    /// the person has chosen one. Until they do, the device's own
    /// recommendation stands (`docs/PLANNING.md` §16.112).
    backup_memory: Option<u32>,
    /// The Argon2id cost this build writes the kept-key blob and
    /// encrypted exports at, where a test has set one. A test sets
    /// Argon2id's minimum, because the parameters are in every header
    /// and a cheap blob opens cheaply; the field is behind `test-hooks`
    /// and no shell has it (`docs/PLANNING.md` §16.119 rule 3).
    #[cfg(feature = "test-hooks")]
    kdf_cost: Option<osk_backup::Cost>,
    /// What backs the shell's secure element, from
    /// [`DisplayInfo::secure`]. Nothing is kept on a device without one.
    secure: SecureHardware,
    /// What the platform said about this device's boot, from
    /// [`DisplayInfo::boot`]. About states it, and an unverified boot on
    /// a device with a secure element is a caution before a key is kept.
    boot: BootState,
    /// Whether the shell says it is holding a kept blob
    /// ([`Event::SecretKept`]).
    secret_kept: bool,
    /// The camera-noise step's live preview, reduced for the
    /// viewfinder, and the full frame behind it that the shutter
    /// hashes. Both are wiped when the step is left.
    create_preview: Option<scan::Preview>,
    /// The luma of the frame the preview was made from, at the size the
    /// shell delivered it.
    create_frame: Option<Zeroizing<Vec<u8>>>,
    /// Whether the camera is on for the Create wizard.
    create_camera: bool,
    /// Whether the device-randomness step's entropy request is out.
    create_entropy_asked: bool,
    /// The blob as it is on the device, once this session wrote or
    /// opened it: what a rewrite starts from.
    kept_blob: Vec<u8>,
    /// The key the blob's keys record is sealed under, sealed under the
    /// session key, once this session wrote or opened it: what lets the
    /// keys on the device follow the keys in memory without asking the
    /// element again (§16.66).
    kept_kek: Option<osk_crypto::Sealed<osk_keep::Kek>>,
    /// The kept-secret channel: the exchange in flight and the pad that
    /// started it.
    keep: keep::Flow,
    /// Blobs written this session, so that two writes never draw the
    /// same salts or nonces from the session key.
    keep_writes: u32,
    /// Encrypted backups made this session, which keeps two of them
    /// from drawing the same salt and nonce (`osk_backup::oskb`).
    backups_made: u32,
    /// True while the Keep screen is the offer that follows adding a
    /// key (§16.65) rather than the row on the key menu. The offer
    /// carries "Not now" beside the hold; the row is reached by a tap
    /// and goes back by the chevron.
    keep_offer: bool,
    /// Attempts left on the stored key, once a wrong PIN has said so.
    /// It outlives the pad, because the Keys row states it too.
    keep_attempts: Option<u8>,
    screen: Screen,
    stack: Vec<Screen>,
    wizard: Option<Wizard>,
    /// The SLIP-39 split now being made, by Add a key › Create SLIP-39
    /// shares or by Backup › SLIP-39 shares. It sits here rather than in
    /// either flow so that the shares exist once
    /// (`docs/PLANNING.md` §16.107).
    shares: SharePlan,
    /// The codex32 strings now being written, by Add a key › Create
    /// Codex32 shares or by Backup › Codex32. It sits here for the
    /// reason the SLIP-39 plan does (`docs/PLANNING.md` §16.109).
    codex32: Codex32Plan,
    /// Splits this session has made, which the next backup's identifier
    /// is drawn from so that two backups of one key are not the same
    /// backup. The identifier is public and stated in every share.
    share_backups: u32,
    detail: DetailState,
    /// The key the "Which key?" Choice has chosen, while the flow that
    /// asked is still running.
    picked_key: Option<usize>,
    /// The wallet whose Keys review sent a person to Add a key, so that
    /// the key they load comes back to it (`docs/PLANNING.md` §16.104
    /// rule 6), and the member that row named.
    add_key_return: Option<AddKeyReturn>,
    /// The transaction a key is being added from the Keys review of
    /// (§16.110 rule 3).
    key_detour: Option<KeyDetour>,
    /// What BIP 129's key record is being written from (§16.110 rule 2).
    bsms: BsmsEntry,
    /// The account a key's Export is showing (§16.110 rule 1).
    account_view: Option<AccountView>,
    /// What a key opened with a passphrase or as a BIP-85 child from a
    /// wallet's not-loaded row gave, when it was not the member that row
    /// named: the statement the step carries until something else is
    /// typed (§16.104 rule 6).
    open_error: Option<String>,
    /// The wallets left out of the blob, by descriptor checksum: a
    /// wallet over a passphrase key this device made and the person
    /// chose not to keep, and any wallet whose "Kept on this device" row
    /// is off (§16.104 rule 7).
    wallets_not_kept: Vec<String>,
    /// The passphrase being typed to open a passphrase key of a loaded
    /// key's words, while that screen is on.
    open_finish: Option<Finish>,
    /// The wallet wizard, set aside while its "Passphrase?" Yes has the
    /// passphrase entry on screen (`docs/PLANNING.md` §16.129 rule 2).
    /// It holds public data only.
    build_parked: Option<WalletWizard>,
    /// The BIP-85 child being chosen, while that screen is on.
    child: Option<ChildFlow>,
    /// The BIP-85 application being derived, while that screen is on
    /// (§16.114).
    bip85: Option<Bip85Flow>,
    /// Tools › Lightning node key, while its screen is up (§16.116).
    lightning: Option<Lightning>,
    /// The vanity grind, while its screens are up (§16.117).
    vanity: Option<vanity::Grind>,
    /// The open Load wizard is that tool's word entry: its words become
    /// a cipher seed to read instead of a key.
    aezeed_entry: bool,
    /// Where Bitcoin Core is told to start scanning for the wallet in
    /// the import file the export writes (§16.114).
    rescan: osk_bip::core_import::Rescan,
    /// The form a silent payment address is shown in (§16.113).
    silent_form: silent::AddressForm,
    /// The user name and domain BIP-353's record is published under.
    dns: silent::Dns,
    /// The transaction a silent payments wallet is checking, while that
    /// flow is on.
    check_payment: Option<silent::Check>,
    /// What became of the Save on that file.
    export_save: sign::Save,
    cache: Vec<CachedAccount>,
    /// The addresses of the wallet whose list is open.
    wallet_addresses: Option<WalletAddresses>,
    /// When the passphrase field must be redrawn to mask its last
    /// character.
    mask_deadline: Option<u64>,
    /// Until when a second Back leaves the app: set by a Back pressed
    /// where nothing is behind (Home, the lock screen, the stored key's
    /// pad), shown as a caption meanwhile, cleared when it runs out or
    /// the screen changes ([`LEAVE_WINDOW_MS`]).
    leave_deadline: Option<u64>,
    /// Whether the terminal screen was reached by leaving (Back twice)
    /// rather than by "Wipe and exit": the first clears memory and keeps
    /// the device's copy, the second forgets that too, and the screen
    /// says which.
    ended_by_leave: bool,
    sign: SignFlow,
    /// The scanner, while the Scan screen is on.
    scan: Option<ScanState>,
    verify: VerifyState,
    /// The message being signed, while its screen is on.
    message: Option<message::SignMessage>,
    /// The signed message that was checked, while its answer is on.
    check: Option<message::CheckMessage>,
    export: ExportState,
    /// The Choice or Compare screen a row opened over the current one.
    overlay: Option<Overlay>,
    inspect: Option<InspectDoc>,
    /// The wallet policies registered this session (`docs/PLANNING.md`
    /// §15 item 8). A wallet is in use until it is forgotten, the
    /// session is wiped, or the device exits; nothing is remembered
    /// across restarts.
    wallets: Vec<WalletPolicy>,
    /// The names given to the wallets on Home, which are public data
    /// and are kept with the wallets on a device that keeps keys.
    names: osk_keep::names::WalletNames,
    /// The name being typed on a wallet's Name screen.
    name_entry: String,
    /// Whether the last frame drew a QR code with its modules visible
    /// (`None`: no QR on screen).
    qr_visible: Option<bool>,
    /// The self-test outcome; `None` until the display is known.
    selftest: Option<osk_selftest::Outcome>,
    /// Whether the shell has a camera. True until it says otherwise, so
    /// the scanner is offered on a shell that has never been asked.
    has_camera: bool,
    /// Whether the shell has a clipboard. True until a write comes back
    /// undone, which is the one answer that says the shell has none: a
    /// read that brings nothing back may only mean the clipboard is
    /// empty. Once false, the Paste and Copy rows are dimmed with the
    /// reason.
    has_clipboard: bool,
    /// A line a screen states for a moment and then stops stating:
    /// "Copied", "No clipboard". The text and the millisecond it ends
    /// at ([`NOTICE_MS`]).
    notice: Option<(String, u64)>,
    /// The session key, PIN, lock state and timers.
    session: Session,
    /// When the app bar's eye stops showing a transcription panel. It
    /// is set only on the screens that carry the eye and cleared by any
    /// other tap and by every screen change, so it never survives
    /// leaving a screen (UX review 2026-09-07, §2.7).
    reveal_until: Option<u64>,
    /// The eye's countdown drawn last, in whole seconds.
    reveal_second: Option<u64>,
    /// A widget the next frame scrolls into view once it has been laid
    /// out: the path editor's checked preset row (`docs/DESIGN.md` §4.6),
    /// or the heading a Learn page opened from the info button starts at
    /// (`docs/PLANNING.md` §16.105), which goes to the top of the view
    /// rather than just inside it.
    scroll_into_view: Option<(Id, Anchor)>,
    /// The Explore workspace; fresh whenever another screen is entered.
    explore: Explore,
    /// The open Load wizard is Explore's word entry: its words return to
    /// Explore instead of becoming a key.
    explore_entry: bool,
    explore_cache: Option<ExploreCache>,
    /// Tools › Word list; fresh whenever the tool is left.
    word_list: WordList,
    /// Tools › Dice passphrase; fresh whenever the tool is left, which
    /// zeroizes the rolls.
    dice: DicePassphrase,
    /// Tools › whichever calculator is open; fresh whenever one is
    /// left, so nothing typed into one is still there in the next.
    calc: Calculator,
    /// Tools › Decode a transaction: the Sign review with no keys, kept
    /// apart from `sign` so that reading a transaction never touches
    /// the flow that signs one.
    decode: SignFlow,
    /// Tools › Compare transactions: the first transaction read and
    /// what differs from the second (`docs/PLANNING.md` §16.111 rule
    /// 4).
    compare_tx: CompareTransactions,
    /// Tools › Notes: the note on screen, whether it was typed, read
    /// from a file, opened out of an encrypted one or taken off the
    /// device. One note is open at a time (`docs/PLANNING.md` §16.112
    /// rule 2).
    note: notes::Note,
    /// The notes kept in the blob's notes record, in the order they
    /// come back (§16.112 pass E3). Empty on a device that keeps
    /// nothing.
    notes_kept: Vec<notes::Note>,
    /// The recovery sheets kept in the same record, each as its three
    /// parts, so that a sheet is still a document after the wallet it
    /// belongs to has gone.
    sheets_kept: Vec<osk_backup::oskb::Sheet>,
    /// The note of each wallet's recovery sheet, by the wallet's
    /// descriptor checksum. Empty until one is written.
    sheet_notes: Vec<(String, notes::Note)>,
    /// A recovery sheet that arrived encrypted and was opened: its
    /// three parts, and the wallet its descriptor would add.
    opened_sheet: Option<osk_backup::oskb::Sheet>,
    /// What the "Which form?" Choice is choosing a form for, and which
    /// row is checked.
    form: Option<(FormFor, Form)>,
    /// The passphrase a note or a sheet is being sealed under, typed
    /// twice.
    seal_pass: crate::pass_entry::PassPair,
    /// The sealed bytes a note or a sheet made, while its Result is on
    /// screen. Ciphertext, so a plain vector.
    sealed: Vec<u8>,
    /// What became of that Result's "Save to file".
    sealed_save: Save,
    /// Whether a QR screen's "Save as PNG" is waiting on the shell. Its
    /// answer is a line on the screen, as Copy's is.
    png_save: Save,
    /// Whether the Start here document has been left once (§6). Until
    /// it is true, it is the screen a device with nothing loaded opens
    /// on. A Tier D device persists nothing, so there it is every run.
    first_run_done: bool,
    /// Whether the address list now open was opened by the first run's
    /// "Check an address": leaving it is what finishes the first run.
    first_run_addresses: bool,
    /// Test hook: make the self-test fail.
    #[cfg(test)]
    fail_selftest: bool,
}

impl OpenSigner {
    /// A fresh application on Home with no keys, for `tier` and `build`.
    pub fn new(tier: AssuranceTier, build: BuildInfo) -> Self {
        OpenSigner {
            tier,
            build,
            canvas: None,
            ctx: None,
            dpi: 160,
            ui: UiState::default(),
            layout: None,
            tree: None,
            dirty: false,
            theme: Theme::DARK,
            commands: VecDeque::new(),
            now_ms: 0,
            keys: Vec::new(),
            network: Network::Mainnet,
            unit: Unit::default(),
            camera_rotation: CameraRotation::default(),
            nonce: osk_psbt::Nonce::default(),
            schnorr: osk_psbt::Schnorr::default(),
            schnorr_draws: 0,
            musig_session: None,
            musig_draws: 0,
            camera_fixed: false,
            memory_mib: None,
            backup_memory: None,
            #[cfg(feature = "test-hooks")]
            kdf_cost: None,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            secret_kept: false,
            create_preview: None,
            create_frame: None,
            create_camera: false,
            create_entropy_asked: false,
            kept_blob: Vec::new(),
            kept_kek: None,
            keep: keep::Flow::new(),
            keep_writes: 0,
            backups_made: 0,
            keep_offer: false,
            keep_attempts: None,
            screen: Screen::Home,
            stack: Vec::new(),
            wizard: None,
            shares: SharePlan::new(),
            codex32: Codex32Plan::new(),
            share_backups: 0,
            detail: DetailState::default(),
            picked_key: None,
            add_key_return: None,
            key_detour: None,
            bsms: BsmsEntry::default(),
            account_view: None,
            open_error: None,
            wallets_not_kept: Vec::new(),
            open_finish: None,
            build_parked: None,
            child: None,
            bip85: None,
            lightning: None,
            vanity: None,
            aezeed_entry: false,
            rescan: osk_bip::core_import::Rescan::Start,
            silent_form: silent::AddressForm::Address,
            dns: silent::Dns::default(),
            check_payment: None,
            export_save: sign::Save::Idle,
            cache: Vec::new(),
            wallet_addresses: None,
            mask_deadline: None,
            leave_deadline: None,
            ended_by_leave: false,
            sign: SignFlow::new(),
            scan: None,
            verify: VerifyState::new(),
            message: None,
            check: None,
            export: ExportState::default(),
            overlay: None,
            inspect: None,
            wallets: Vec::new(),
            names: osk_keep::names::WalletNames::new(),
            name_entry: String::new(),
            qr_visible: None,
            selftest: None,
            has_camera: true,
            has_clipboard: true,
            notice: None,
            session: Session::new(&[]),
            reveal_until: None,
            reveal_second: None,
            scroll_into_view: None,
            explore: Explore::new(Network::Mainnet),
            word_list: WordList::new(),
            calc: Calculator::new(),
            decode: SignFlow::new(),
            compare_tx: CompareTransactions::new(),
            note: notes::Note::new(),
            notes_kept: Vec::new(),
            sheets_kept: Vec::new(),
            sheet_notes: Vec::new(),
            opened_sheet: None,
            form: None,
            seal_pass: crate::pass_entry::PassPair::new(),
            sealed: Vec::new(),
            sealed_save: Save::Idle,
            png_save: Save::Idle,
            dice: DicePassphrase::new(),
            explore_entry: false,
            explore_cache: None,
            first_run_done: false,
            first_run_addresses: false,
            #[cfg(test)]
            fail_selftest: false,
        }
    }

    /// The Argon2id cost the kept-key blob is written at: what the
    /// device states, unless a test has set its own.
    fn keep_cost(&self) -> osk_backup::Cost {
        #[cfg(feature = "test-hooks")]
        if let Some(cost) = self.kdf_cost {
            return cost;
        }
        osk_backup::DEVICE_PARAMS
    }

    /// The Argon2id cost an encrypted export is written at: the memory
    /// the person chose, unless a test has set its own.
    fn export_cost(&self) -> osk_backup::Cost {
        #[cfg(feature = "test-hooks")]
        if let Some(cost) = self.kdf_cost {
            return cost;
        }
        backup_cost(self.backup_memory_kib())
    }

    /// Write the blob and every encrypted export at `cost` instead of
    /// the device's. Argon2id's parameters are in every header, so what
    /// is written at this cost opens at it (`docs/PLANNING.md`
    /// §16.119 rule 3).
    #[cfg(feature = "test-hooks")]
    pub fn set_kdf_cost(&mut self, cost: osk_backup::Cost) {
        self.kdf_cost = Some(cost);
    }

    // ----- inspection (non-secret) -----

    /// The screen showing.
    pub fn screen(&self) -> ScreenKind {
        if self.selftest_failed() {
            return ScreenKind::SelfTestFailed;
        }
        if self.boot_refused() {
            return ScreenKind::BootRefused;
        }
        if self.session.is_locked() {
            return ScreenKind::Lock;
        }
        if self.front_door() {
            return ScreenKind::StoredKey;
        }
        // §4.2's Setting is a screen of its own over Settings, and the
        // one overlay a flow can be left sitting on.
        if matches!(self.overlay, Some(Overlay::Setting(_))) {
            return ScreenKind::Setting;
        }
        // The file list is a screen of its own over the screen that
        // asked for a file, and the flow underneath is still waiting.
        if matches!(self.overlay, Some(Overlay::Files(_))) {
            return ScreenKind::Files;
        }
        // §16.105: a page opened from the info button is drawn over
        // everything below it — a wizard, the cosigner scanner — and is
        // the screen a person is on until the chevron leaves it.
        if matches!(self.screen, Screen::LearnTopic(..)) {
            return ScreenKind::LearnPage;
        }
        // The scanner opened for a cosigner sits over the builder, and
        // is the screen a person is on while it is up.
        if self.building_scan() {
            return ScreenKind::Scan;
        }
        match &self.wizard {
            Some(Wizard::Load(_)) => ScreenKind::Load,
            Some(Wizard::Build(_)) => ScreenKind::Build,
            Some(Wizard::Create(_)) => ScreenKind::Create,
            Some(Wizard::Backup(_)) => ScreenKind::Backup,
            None if self.start_here_shown() => ScreenKind::StartHere,
            None => self.screen.kind(),
        }
    }

    /// Whether Home is drawing the first run's document instead of its
    /// list: nothing is loaded and the document has never been left.
    pub(crate) fn start_here_shown(&self) -> bool {
        self.screen == Screen::Home
            && !self.first_run_done
            && self.home_is_empty()
            && self.wizard.is_none()
    }

    /// Whether the first run is over, which is what the Start here
    /// document's chevron, a loaded key and a checked address each end.
    pub fn first_run_done(&self) -> bool {
        self.first_run_done
    }

    /// The unit amounts are shown in (§4.7).
    pub fn unit(&self) -> Unit {
        self.unit
    }

    /// The self-test outcome, once it has run.
    pub fn selftest(&self) -> Option<osk_selftest::Outcome> {
        self.selftest
    }

    fn selftest_failed(&self) -> bool {
        matches!(self.selftest, Some(Err(_)))
    }

    /// The Sign flow's stage, while the Sign screen is showing.
    pub fn sign_stage(&self) -> Option<Stage> {
        (self.screen == Screen::Sign && self.wizard.is_none()).then(|| self.sign.stage())
    }

    /// The message being signed, while its screen is on.
    pub fn signing_message(&self) -> Option<&message::SignMessage> {
        self.message.as_ref()
    }

    /// The signed message that was checked, while its answer is on.
    pub fn checked_message(&self) -> Option<&message::CheckMessage> {
        self.check.as_ref()
    }

    /// The address the message screen signs for.
    pub fn message_address_shown(&self) -> Option<String> {
        self.message.as_ref().map(|f| self.message_address(f))
    }

    /// The inspection of the loaded PSBT, while one is loaded.
    pub fn sign_inspection(&self) -> Option<&Inspection> {
        self.sign.inspection()
    }

    /// How many signatures the Sign flow's last pass wrote, once it has
    /// signed; `None` before it has.
    pub fn sign_outcome_signatures(&self) -> Option<usize> {
        Some(self.sign.outcome()?.signatures.len())
    }

    /// The id of the transaction the Sign flow's last pass finished,
    /// when it finished one.
    pub fn sign_outcome_txid(&self) -> Option<osk_psbt::bitcoin::Txid> {
        self.sign.outcome()?.txid
    }

    /// The PSBT the Sign flow's last pass wrote: signed, and finalized
    /// where the pass left every input satisfied. It is what the QR
    /// page carries and what a coordinator is handed.
    pub fn sign_outcome_psbt(&self) -> Option<&[u8]> {
        Some(&self.sign.outcome()?.psbt)
    }

    /// The other shares chosen to sign at the first location.
    pub fn sign_others(&self) -> Vec<u32> {
        self.sign.others().to_vec()
    }

    /// Whether the hold-to-sign button is live.
    pub fn sign_enabled(&self) -> bool {
        self.sign.can_sign()
    }

    /// The QR page's mode, while the Sign result shows as a QR.
    pub fn sign_qr_mode(&self) -> Option<QrMode> {
        (self.sign_stage() == Some(Stage::Wizard(sign::Step::Qr)))
            .then(|| self.sign.qr_mode())
            .flatten()
    }

    /// Whether one static code is readable on this display, which is
    /// what the QR page opened on (UX review 2026-09-07, §2.8).
    pub fn sign_static_qr_scans(&self) -> bool {
        self.sign.static_qr_scans()
    }

    /// `(part shown, fragments)` of the animated UR on the QR page.
    pub fn sign_qr_part(&self) -> Option<(usize, usize)> {
        (self.sign_qr_mode() == Some(QrMode::Ur))
            .then(|| self.sign.qr_part())
            .flatten()
    }

    /// The scanner's stage, while the Scan screen is on.
    pub fn scan_stage(&self) -> Option<ScanStage> {
        (self.screen == Screen::Scan && (self.wizard.is_none() || self.building_scan()))
            .then(|| self.scan.as_ref().map(ScanState::stage))
            .flatten()
    }

    /// Where the wallet builder is, for tests.
    pub fn build_step(&self) -> Option<build::Step> {
        match &self.wizard {
            Some(Wizard::Build(w)) => Some(w.step()),
            _ => None,
        }
    }

    /// `(fragments resolved, fragments)` of the multi-part UR the scanner
    /// is collecting.
    pub fn scan_progress(&self) -> Option<(usize, usize)> {
        self.scan.as_ref().and_then(ScanState::progress)
    }

    /// The size in pixels of the camera preview the scanner is holding,
    /// or `None` before the first frame and where there is no camera.
    pub fn scan_preview(&self) -> Option<(u16, u16)> {
        self.scan
            .as_ref()
            .and_then(ScanState::preview)
            .map(|p| (p.width(), p.height()))
    }

    /// The scanner's error line, if any.
    pub fn scan_error(&self) -> Option<&str> {
        self.scan.as_ref().and_then(ScanState::error)
    }

    /// Verify › address: where the screen is, while it is on.
    pub fn verify_stage(&self) -> Option<VerifyStage> {
        (self.screen == Screen::Verify && self.wizard.is_none()).then(|| self.verify.stage())
    }

    /// Verify › address: the text in the field, for a test that taps
    /// the address out key by key.
    pub fn verify_input(&self) -> &str {
        self.verify.input()
    }

    /// Verify › address: the last result.
    pub fn verify_result(&self) -> Option<&AddressResult> {
        self.verify.result()
    }

    /// The title of the Inspect document, while it is on.
    pub fn inspect_title(&self) -> Option<&'static str> {
        (self.screen == Screen::Inspect)
            .then(|| self.inspect.as_ref().map(|d| d.title))
            .flatten()
    }

    /// Whether the last frame drew a QR code with its modules visible;
    /// `None` when no QR widget was on screen. A secret code is visible
    /// only while its hold button is held (UX.md §5).
    pub fn qr_visible(&self) -> Option<bool> {
        self.qr_visible
    }

    // ----- explore (non-secret) -----

    /// Whether Explore holds typed words.
    pub fn explore_has_input(&self) -> bool {
        self.explore.has_input()
    }

    /// What Explore derives from, while it is on.
    pub fn explore_source(&self) -> Option<Source> {
        (self.screen == Screen::Explore).then(|| self.explore.source())
    }

    /// Which of Explore's screens is showing, while the area is on.
    pub fn explore_step(&self) -> Option<explore::Step> {
        (self.screen == Screen::Explore).then(|| self.explore.step())
    }

    /// Whether the Explore screen on display is showing its secret: a
    /// finger on the panel, or the app bar's eye running.
    pub fn explore_words_revealed(&self) -> bool {
        self.screen == Screen::Explore && self.wizard.is_none() && self.revealed(ids::CREATE_REVEAL)
    }

    /// The master fingerprint of what Explore derives from.
    pub fn explore_fingerprint(&self) -> Option<Fingerprint> {
        self.explore_master().map(MasterKey::fingerprint)
    }

    /// The path the explorer last derived, `m/84h/0h/0h/0/0`.
    pub fn explore_path(&self) -> Option<String> {
        (self.screen == Screen::Explore).then(|| text::path(self.explore.applied_path()))
    }

    /// The inline error under the path field, if the text does not
    /// parse.
    pub fn explore_path_error(&self) -> Option<&'static str> {
        let s = self.strings();
        self.explore.path_error().map(|e| e.message(s))
    }

    /// The addresses at the leaf of the explored path, as drawn last.
    pub fn explore_addresses(&self) -> Vec<(ScriptType, String)> {
        self.explore_cache
            .as_ref()
            .map_or_else(Vec::new, |c| c.addresses.clone())
    }

    /// The account key of the Encodings section, as drawn last: the
    /// xpub and the SLIP-132 form when the purpose has one.
    pub fn explore_account(&self) -> Option<(String, Option<String>)> {
        self.explore_cache.as_ref().map(|c| {
            (
                c.account_xpub.clone(),
                c.account_slip132.as_ref().map(|(_, s)| s.clone()),
            )
        })
    }

    /// Fingerprints of the loaded keys, in key order.
    pub fn fingerprints(&self) -> Vec<Fingerprint> {
        self.keys.iter().map(|k| k.fingerprint).collect()
    }

    /// Which registered wallet is the single-sig wallet of the key at
    /// `key` paying to `script`, if one was added. A single-sig wallet
    /// is explicit (`docs/PLANNING.md` §16.104 rule 2), so this answers
    /// whether it exists rather than making one up.
    pub fn single_sig_wallet(&self, key: usize, script: ScriptType) -> Option<usize> {
        let fp = self.keys.get(key)?.fingerprint;
        self.wallets.iter().position(|p| {
            p.template() == (Template::Single { script })
                && p.keys().first().and_then(PolicyKey::fingerprint) == Some(fp)
        })
    }

    /// How many wallets are registered.
    pub fn wallet_count(&self) -> usize {
        self.wallets.len()
    }

    /// The string the Export screen now offers, in the format its row
    /// names. Public data: a descriptor, an account key or a
    /// coordinator's record.
    pub fn export_string(&self) -> Option<String> {
        match self.screen {
            Screen::Export(_) | Screen::KeyExport(..) => self.export_value(self.export_owner()),
            _ => None,
        }
    }

    /// The policy of the wallet at `i` in the Wallets list. Public data:
    /// a descriptor over account extended public keys.
    pub fn wallet_policy(&self, i: usize) -> Option<&WalletPolicy> {
        self.wallets.get(i)
    }

    /// The network setting.
    pub fn network(&self) -> Network {
        self.network
    }

    /// The Load wizard's step, while it is open.
    pub fn load_step(&self) -> Option<Step> {
        match &self.wizard {
            Some(Wizard::Load(w)) => Some(w.step()),
            _ => None,
        }
    }

    /// How many words the entry on screen has accepted, the Load
    /// wizard's or a Seed XOR part's. The words themselves are a secret
    /// and masked on screen; the count is not, and it is what says
    /// whether a tap took a candidate.
    pub fn load_words_accepted(&self) -> usize {
        match &self.wizard {
            Some(Wizard::Load(w)) => w.committed_indices().count(),
            _ => match self.gatherer() {
                Some(w) if w.step() == create::Step::XorPart => {
                    w.entry().committed_indices().count()
                }
                _ => 0,
            },
        }
    }

    /// After a checksum failure: the 1-based word positions the wizard
    /// suspects.
    pub fn load_suspects(&self) -> Vec<u8> {
        match &self.wizard {
            Some(Wizard::Load(w)) => w.suspects().map(|p| p as u8 + 1).collect(),
            _ => Vec::new(),
        }
    }

    /// The Create wizard's step, while it is open. A Seed XOR split
    /// gathers its random parts in a wizard of the same kind, so its
    /// steps are reported here too.
    pub fn create_step(&self) -> Option<create::Step> {
        self.gatherer().map(CreateWizard::step)
    }

    /// The Create wizard's sanity flags, while it is open.
    pub fn create_warnings(&self) -> Option<osk_entropy::Warnings> {
        self.gatherer().map(CreateWizard::warnings)
    }

    /// The Create wizard's entry count, while it is open.
    pub fn create_entries(&self) -> Option<usize> {
        self.gatherer().map(CreateWizard::entry_len)
    }

    /// The SLIP-39 split's step, while one is being made. A random
    /// value being gathered reports [`ShareStep::Gather`], and the
    /// gatherer's own step is [`create_step`](Self::create_step).
    pub fn split_step(&self) -> Option<ShareStep> {
        let running = match &self.wizard {
            Some(Wizard::Create(w)) => w.is_slip39() && self.shares.is_running(),
            Some(Wizard::Backup(b)) => b.step() == BackupStep::Shares,
            _ => false,
        };
        running.then(|| self.shares.step())
    }

    /// The codex32 plan's step, while one is being written. A run of
    /// the source being gathered reports [`Codex32Step::Gather`], and
    /// the gatherer's own step is [`create_step`](Self::create_step).
    pub fn codex32_step(&self) -> Option<Codex32Step> {
        let running = match &self.wizard {
            Some(Wizard::Create(w)) => w.is_codex32() && self.codex32.is_running(),
            Some(Wizard::Backup(b)) => b.step() == BackupStep::Codex32,
            _ => false,
        };
        running.then(|| self.codex32.step())
    }

    /// The Backup flow's step, while it is open.
    pub fn backup_step(&self) -> Option<BackupStep> {
        match &self.wizard {
            Some(Wizard::Backup(b)) => Some(b.step()),
            _ => None,
        }
    }

    /// Whether key `key` has passed the backup quiz.
    pub fn backup_verified(&self, key: usize) -> Option<bool> {
        self.keys.get(key).map(|k| k.backup_verified)
    }

    /// Whether key `key` still holds its words.
    pub fn has_mnemonic(&self, key: usize) -> Option<bool> {
        self.keys.get(key).map(LoadedKey::has_mnemonic)
    }

    /// Whether key `key` carries a passphrase.
    pub fn has_passphrase(&self, key: usize) -> Option<bool> {
        self.keys.get(key).map(|k| k.has_passphrase)
    }

    /// Whether key `key` was derived from another loaded key, and so
    /// lives in memory for this session alone.
    pub fn is_derived(&self, key: usize) -> Option<bool> {
        self.keys.get(key).map(|k| k.derived)
    }

    /// Whether key `key`'s seed is sealed under the session key (rather
    /// than plaintext because the session key is weak).
    pub fn is_sealed(&self, key: usize) -> Option<bool> {
        self.keys.get(key).map(LoadedKey::is_sealed)
    }

    // ----- session (non-secret) -----

    /// Whether the lock screen is showing.
    pub fn is_locked(&self) -> bool {
        self.session.is_locked()
    }

    /// Whether a session PIN has been set.
    pub fn has_pin(&self) -> bool {
        self.session.has_pin()
    }

    /// Wrong PINs left before the session wipes.
    pub fn attempts_left(&self) -> u8 {
        self.session.attempts_left()
    }

    /// Whether the session key was built without shell entropy.
    pub fn session_weak(&self) -> bool {
        self.session.is_weak()
    }

    /// Milliseconds until the auto-lock fires, while there is something
    /// to lock (keys loaded, not locked).
    pub fn lock_in_ms(&self) -> Option<u64> {
        (self.has_secrets() && !self.session.is_locked())
            .then(|| self.session.lock_deadline().saturating_sub(self.now_ms))
    }

    /// Milliseconds until the auto-wipe fires, while keys are loaded and
    /// the timer is not "never".
    pub fn wipe_in_ms(&self) -> Option<u64> {
        if !self.has_secrets() {
            return None;
        }
        self.session
            .wipe_deadline()
            .map(|d| d.saturating_sub(self.now_ms))
    }

    /// The auto-lock timeout.
    pub fn lock_after_ms(&self) -> u64 {
        self.session.lock_after_ms()
    }

    /// The auto-wipe timeout; `None` is never.
    pub fn wipe_after_ms(&self) -> Option<u64> {
        self.session.wipe_after_ms()
    }

    /// The seed the PIN pad on a PIN step is shuffled with. The repeat
    /// step shuffles differently from the first, so that the second
    /// entry is not the same sequence of taps as the first (UX review
    /// 2026-09-07, §3.2).
    pub(crate) fn pin_scramble(&self, repeat: bool) -> Option<u32> {
        let seed = self.session.scramble_seed()?;
        Some(if repeat { seed ^ 0x9E37_79B9 } else { seed })
    }

    /// Whether the PIN pad is scrambled.
    pub fn scramble_pin(&self) -> bool {
        self.session.scramble_pin()
    }

    /// How far every camera frame is turned before it is read. A shell
    /// whose frames are always upright turns them not at all, whatever
    /// a settings file carried over from another device says (§4.9).
    pub fn camera_rotation(&self) -> CameraRotation {
        if self.camera_fixed {
            CameraRotation::Deg0
        } else {
            self.camera_rotation
        }
    }

    /// Which RFC 6979 nonce every ECDSA signature this device makes
    /// uses (§16.38).
    pub fn nonce(&self) -> osk_psbt::Nonce {
        self.nonce
    }

    /// Where the auxiliary randomness of every Schnorr signature this
    /// device makes comes from (security review 2026-09-11, L3).
    pub fn schnorr(&self) -> osk_psbt::Schnorr {
        self.schnorr
    }

    /// The auxiliary randomness one signing call signs under: nothing in
    /// the deterministic mode, and otherwise 32 bytes drawn from the
    /// session key with a count of the draws, so that no two calls of a
    /// session sign under the same bytes.
    fn schnorr_aux(&mut self) -> osk_psbt::Aux {
        if self.schnorr == osk_psbt::Schnorr::Deterministic {
            return osk_psbt::Aux::Deterministic;
        }
        self.schnorr_draws = self.schnorr_draws.wrapping_add(1);
        let mut label = [0u8; 16];
        label[..12].copy_from_slice(b"osk-aux-rand");
        label[12..].copy_from_slice(&self.schnorr_draws.to_le_bytes());
        let derived = self.session.key().derive(&label);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&derived.expose()[..32]);
        osk_psbt::Aux::Fresh(bytes)
    }

    /// The seed a new MuSig2 session draws its secret nonces from: the
    /// session key with a label and a count of the draws, the way
    /// [`schnorr_aux`](Self::schnorr_aux) draws its bytes, so that every
    /// session of a boot draws from different bytes and the shell is
    /// asked for nothing new (§16.100).
    fn musig_seed(&mut self) -> [u8; 32] {
        self.musig_draws = self.musig_draws.wrapping_add(1);
        let mut label = [0u8; 18];
        label[..14].copy_from_slice(b"osk-musig-rand");
        label[14..].copy_from_slice(&self.musig_draws.to_le_bytes());
        let derived = self.session.key().derive(&label);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&derived.expose()[..32]);
        bytes
    }

    /// Whether this device holds a MuSig2 secret nonce, which is what
    /// Home's session badge says (§4.1).
    pub fn musig_session_open(&self) -> bool {
        self.musig_session.is_some()
    }

    /// Whether the camera's frames are always upright, so the rotation
    /// is neither offered nor applied.
    pub fn camera_fixed(&self) -> bool {
        self.camera_fixed
    }

    /// The PIN pad's scramble seed, when scrambling is on. Public data:
    /// it decides the key order and nothing else.
    pub fn pin_scramble_seed(&self) -> Option<u32> {
        self.session.scramble_seed()
    }

    /// The rectangle of key `input` on the on-screen keyboard `keyboard`
    /// in the last drawn frame, for tapping PIN digits in tests and
    /// review scripts (PIN entry is touch-only, `docs/PLANNING.md` §4.5).
    pub fn key_rect(&self, keyboard: Id, input: KeyInput) -> Option<Rect> {
        let layout = self.layout.as_ref()?;
        let placed = layout.placed(keyboard)?;
        let Some(HitTarget::Keyboard {
            kind,
            enabled,
            scramble,
            ..
        }) = placed.hit
        else {
            return None;
        };
        keyboard::keys(
            kind,
            placed.rect,
            &layout.ctx,
            enabled,
            scramble,
            self.ui.modifiers(),
        )
        .into_iter()
        .find(|k| k.input == input && k.enabled)
        .map(|k| k.rect)
    }

    /// Every key the on-screen keyboard `keyboard` drew in the last
    /// frame, dead keys included: what the pad shows, whether or not a
    /// tap on it does anything.
    pub fn keycaps(&self, keyboard: Id) -> Vec<KeyInput> {
        let Some(layout) = self.layout.as_ref() else {
            return Vec::new();
        };
        let Some(placed) = layout.placed(keyboard) else {
            return Vec::new();
        };
        let Some(HitTarget::Keyboard {
            kind,
            enabled,
            scramble,
            ..
        }) = placed.hit
        else {
            return Vec::new();
        };
        keyboard::keys(
            kind,
            placed.rect,
            &layout.ctx,
            enabled,
            scramble,
            self.ui.modifiers(),
        )
        .into_iter()
        .map(|k| k.input)
        .collect()
    }

    /// The cell rectangle of the `n`-th candidate of strip `id`, for a
    /// script or a test that taps one. `None` when the strip is not on
    /// screen or has no such cell.
    pub fn candidate_rect(&self, strip: Id, n: usize) -> Option<Rect> {
        let layout = self.layout.as_ref()?;
        let placed = layout.placed(strip)?;
        let Some(HitTarget::Candidates { count, cells, .. }) = placed.hit else {
            return None;
        };
        (n < usize::from(count)).then(|| {
            let cell = (placed.rect.w / i32::from(cells.max(count))).max(1);
            Rect::new(
                placed.rect.x + n as i32 * cell,
                placed.rect.y,
                cell,
                placed.rect.h,
            )
        })
    }

    /// The cell that turns the candidate strip's page, when the strip
    /// has one. A script taps it without knowing which row it is on.
    pub fn candidate_more_rect(&self) -> Option<Rect> {
        fn walk(node: &Node) -> Option<Id> {
            if let Some(Widget::CandidateStrip { id, more: true, .. }) = node.as_widget() {
                return Some(*id);
            }
            node.children.iter().find_map(walk)
        }
        let id = walk(&self.build())?;
        let placed = self.layout.as_ref()?.placed(id)?;
        let Some(HitTarget::Candidates { count, .. }) = placed.hit else {
            return None;
        };
        self.candidate_rect(id, usize::from(count) - 1)
    }

    /// What the quiz screen shows, while a quiz is on.
    pub fn quiz_view(&self) -> Option<QuizView> {
        let (q, list): (&Quiz, EntryList) = match &self.wizard {
            Some(Wizard::Create(w)) if w.is_slip39() => (self.shares.quiz()?, EntryList::Slip39),
            Some(Wizard::Create(w)) => (w.quiz()?, EntryList::Bip39(w.language())),
            Some(Wizard::Backup(b)) if b.step() == BackupStep::Shares => {
                (self.shares.quiz()?, EntryList::Slip39)
            }
            Some(Wizard::Backup(b)) => {
                let lang = self.backup_words(b, Mnemonic::language)?;
                (b.quiz()?, EntryList::Bip39(lang))
            }
            _ => return None,
        };
        let (done, total) = q.progress();
        Some(QuizView {
            word_number: q.position() + 1,
            choices: views::quiz::choice_labels(q, list),
            done,
            total,
            state: q.state(),
            helper: q.helper(),
            message: views::quiz::wrong_message(q, self.strings()),
        })
    }

    /// Whether `id` is under a finger right now — a secret panel or a
    /// hold button — for tests of the reveal gesture.
    pub fn is_held(&self, id: Id) -> bool {
        self.ui.is_held(id)
    }

    /// Seconds left of the app bar's eye reveal, while one is running.
    /// `None` means the panel is masked unless a finger is on it.
    pub fn reveal_left_s(&self) -> Option<u64> {
        self.reveal_until
            .filter(|&t| t > self.now_ms)
            .map(|t| (t - self.now_ms).div_ceil(1000))
    }

    /// Whether the panel `id` is showing: a finger on it, or the eye
    /// within its [`REVEAL_MS`]. Only the transcription screens ask.
    fn revealed(&self, id: Id) -> bool {
        self.ui.is_held(id) || self.reveal_left_s().is_some()
    }

    /// The share of the eye's reveal still to run, which draws the eye
    /// as a ring that empties clockwise (`docs/DESIGN.md` §4.10). `None`
    /// while no reveal is running: there is no countdown text anywhere.
    fn reveal_ring(&self) -> Option<f32> {
        self.reveal_left_s()
            .map(|left| osk_ui::components::reveal_remaining(left as u32))
    }

    /// What the eye's reveal is tied to: the screen, the step of the
    /// wizard on it, and Explore's own step. The word page is not part
    /// of it, so turning the page keeps the words on the screen.
    fn reveal_scope(&self) -> (ScreenKind, Option<u8>, u8, u8) {
        let step = match &self.wizard {
            Some(Wizard::Load(w)) => Some(w.step() as u8),
            // Add a wallet holds no secret until its FROST deal, whose
            // words are the one thing on it a reveal covers.
            Some(Wizard::Build(w)) => Some(0x20 | w.step() as u8),
            Some(Wizard::Create(w)) => Some(0x40 | w.step() as u8),
            Some(Wizard::Backup(b)) => Some(0x80 | b.step() as u8),
            // The dice passphrase's own steps are a run like a wizard's:
            // the rolls, the passphrase and the confirm each end a look
            // at the one before.
            None if self.screen == Screen::DicePassphrase => Some(0xC0 | self.dice.step() as u8),
            None => None,
        };
        // §4.3: "Leaving the screen or accepting a word masks it." The
        // number of words accepted is part of the scope, so the next
        // accepted word ends the reveal by itself.
        let accepted = match &self.wizard {
            Some(Wizard::Load(w)) => w.committed_indices().count(),
            None if self.screen == Screen::DicePassphrase => self.dice.words().len(),
            _ => 0,
        };
        (
            self.screen(),
            step,
            self.explore.step().scope()
                | (self.lightning.as_ref().map_or(0, |l| l.step().scope() + 1) << 4),
            accepted as u8,
        )
    }

    /// Starts or stops the eye's reveal.
    fn toggle_reveal(&mut self) {
        self.reveal_until = match self.reveal_left_s() {
            Some(_) => None,
            None => Some(self.now_ms + REVEAL_MS),
        };
    }

    /// Every button and row label the current screen draws, in tree
    /// order. Non-secret by construction: a label comes from
    /// [`Strings`], never from a value. Tests read it to assert what a
    /// screen offers.
    pub fn labels(&self) -> Vec<String> {
        fn walk(node: &Node, out: &mut Vec<String>) {
            match node.as_widget() {
                Some(Widget::Button { label, .. } | Widget::HoldButton { label, .. }) => {
                    out.push(label.clone());
                }
                Some(Widget::ListRow { title, .. }) => out.push(title.clone()),
                _ => {}
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// Every string of characters the current screen draws, in tree
    /// order: text, labels, row titles and their trailing values, chips,
    /// badges and chunked strings. Tests read it to assert that
    /// something is *not* on the screen — that a wrong quiz answer never
    /// prints the right word, that a masked panel prints only bullets.
    pub fn texts(&self) -> Vec<String> {
        fn walk(node: &Node, out: &mut Vec<String>) {
            match node.as_widget() {
                // A masked chunked string draws nothing; only its
                // rectangle is kept (§4.10), so there is nothing on the
                // screen to report.
                Some(Widget::ChunkedString { masked: true, .. }) => {}
                Some(
                    Widget::Text { text, .. }
                    | Widget::ChunkedString { text, .. }
                    | Widget::Button { label: text, .. }
                    | Widget::HoldButton { label: text, .. }
                    | Widget::Chip { label: text, .. }
                    | Widget::Badge { label: text, .. },
                ) => out.push(text.clone()),
                // A row's value is its subtitle where §4.1 puts the
                // value under the label, and its trailing text where a
                // row still carries one beside it.
                Some(Widget::ListRow {
                    title,
                    subtitle,
                    trailing,
                    ..
                }) => {
                    out.push(title.clone());
                    for t in [subtitle, trailing].into_iter().flatten() {
                        out.push(t.clone());
                    }
                }
                Some(Widget::LabelledValue { label, value, .. }) => {
                    out.push(label.clone());
                    out.push(value.clone());
                }
                // A hub tile is its word and, on Keys, the count in its
                // corner.
                Some(Widget::Tile { label, badge, .. }) => {
                    out.push(label.clone());
                    if let Some(badge) = badge {
                        out.push(badge.clone());
                    }
                }
                _ => {}
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// Every row the current screen draws with a leading mark, as its
    /// label and the mark. §4.4's glyph rule is what a person reads off
    /// a wallet row — the key for a wallet this device can sign for, the
    /// eye for one it can only watch — so a test reads the same pairs.
    pub fn row_glyphs(&self) -> Vec<(String, Icon)> {
        fn walk(node: &Node, out: &mut Vec<(String, Icon)>) {
            if let Some(Widget::ListRow {
                title,
                icon: Some(icon),
                ..
            }) = node.as_widget()
            {
                out.push((title.clone(), *icon));
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// Every value the current screen draws with a glyph before it, as
    /// the value and the glyph: a fingerprint and the fingerprint glyph
    /// (`docs/PLANNING.md` §16.131). A row's leading mark is
    /// [`row_glyphs`](Self::row_glyphs); this is the mark inside a value.
    pub fn value_glyphs(&self) -> Vec<(String, Icon)> {
        fn walk(node: &Node, out: &mut Vec<(String, Icon)>) {
            match node.as_widget() {
                Some(Widget::ListRow {
                    subtitle: Some(value),
                    value_below: Some(v),
                    ..
                }) => {
                    if let Some(g) = v.glyph {
                        out.push((value.clone(), g));
                    }
                }
                Some(
                    Widget::Text {
                        text: value,
                        glyph: Some(g),
                        ..
                    }
                    | Widget::Chip {
                        label: value,
                        glyph: Some(g),
                        ..
                    }
                    | Widget::LabelledValue {
                        value,
                        glyph: Some(g),
                        ..
                    },
                ) => out.push((value.clone(), *g)),
                _ => {}
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// Every row the current screen draws dimmed, with the reason under
    /// its label. A dimmed row with no reason a person can act on is the
    /// most confusing thing on the first run (UX review 2026-09-07,
    /// §2.3), so a test walks every screen that has one.
    pub fn dimmed_rows(&self) -> Vec<DimmedRow> {
        fn walk(node: &Node, out: &mut Vec<DimmedRow>) {
            if let Some(Widget::ListRow {
                title,
                trailing,
                subtitle,
                enabled: false,
                ..
            }) = node.as_widget()
            {
                // §4.11 puts the reason under the label, so it is the
                // row's second line; a reason beside the label arrives
                // as the trailing text instead, and the test tells them
                // apart.
                out.push(DimmedRow {
                    label: title.clone(),
                    reason: subtitle.clone().or_else(|| trailing.clone()),
                    below: subtitle.is_some(),
                });
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// The labels of every tile the current screen draws dimmed (§4.1,
    /// §4.11): a way in this shell cannot serve.
    pub fn dimmed_tiles(&self) -> Vec<String> {
        fn walk(node: &Node, out: &mut Vec<String>) {
            if let Some(Widget::Tile {
                label,
                enabled: false,
                ..
            }) = node.as_widget()
            {
                out.push(label.clone());
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// The labels of every row the current screen draws with the accent
    /// check (§4.2). A Choice has one; the path editor's purpose group
    /// has one, and none where the typed path matches no preset (§4.6).
    pub fn checked_rows(&self) -> Vec<String> {
        fn walk(node: &Node, out: &mut Vec<String>) {
            if let Some(Widget::ListRow {
                title,
                selected: true,
                ..
            }) = node.as_widget()
            {
                out.push(title.clone());
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// The wallet export's QR as the page opened it: whether it animates,
    /// the largest QR version this display keeps above the pitch floor at
    /// the class's side, and the string it carries (§4.9).
    pub fn export_qr_shape(&self) -> Option<(bool, u8, String)> {
        if !matches!(self.screen, Screen::Export(_) | Screen::KeyExport(..)) {
            return None;
        }
        Some((
            self.export.animated,
            self.export.version,
            self.export_value(self.export_owner())?,
        ))
    }

    /// The same three facts for the Sign result's QR page.
    pub fn sign_qr_payload(&self) -> Option<(bool, u8, Vec<u8>)> {
        Some((
            self.sign_qr_mode() == Some(QrMode::Ur),
            self.sign.qr_version(),
            self.sign.ur_payload()?.to_vec(),
        ))
    }

    /// The words the candidate strips of the current screen offer, in
    /// order (§4.3). Tests read it to assert what ✓ would accept.
    pub fn candidates(&self) -> Vec<String> {
        fn walk(node: &Node, out: &mut Vec<String>) {
            match node.as_widget() {
                Some(Widget::CandidateStrip { words, .. }) => out.extend(words.iter().cloned()),
                Some(Widget::Chip { label, .. }) => out.push(label.clone()),
                _ => {}
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// Whether the candidate strip's last cell pages rather than
    /// carrying a word (§4.3). Tests read it to know which cells accept.
    pub fn candidate_more(&self) -> bool {
        fn walk(node: &Node) -> bool {
            if let Some(Widget::CandidateStrip { more: true, .. }) = node.as_widget() {
                return true;
            }
            node.children.iter().any(walk)
        }
        walk(&self.build())
    }

    /// Every chip the current screen draws dimmed, by label: a choice
    /// that is still offered but is a poor one here (UX review
    /// 2026-09-07, §2.8).
    pub fn dimmed_chips(&self) -> Vec<String> {
        fn walk(node: &Node, out: &mut Vec<String>) {
            if let Some(Widget::Chip {
                label,
                dimmed: true,
                ..
            }) = node.as_widget()
            {
                out.push(label.clone());
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// Every chip the current screen draws, with its label. Layout
    /// tests use it to check that a chip is wide enough for what is
    /// written on it (UX review 2026-09-07, finding 5).
    pub fn chip_labels(&self) -> Vec<(Id, String)> {
        fn walk(node: &Node, out: &mut Vec<(Id, String)>) {
            if let Some(Widget::Chip {
                id: Some(id),
                label,
                ..
            }) = node.as_widget()
            {
                out.push((*id, label.clone()));
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// The ids of every hold button the current screen draws. A hold is
    /// only ever the last step of an action that cannot be undone
    /// (`docs/PLANNING.md` §16.27), which the layout tests assert.
    pub fn hold_buttons(&self) -> Vec<Id> {
        fn walk(node: &Node, out: &mut Vec<Id>) {
            if let Some(Widget::HoldButton { id, .. }) = node.as_widget() {
                out.push(*id);
            }
            for c in &node.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.build(), &mut out);
        out
    }

    /// The label of the `wide` sidebar's selected entry, for a test that
    /// asserts which area a screen belongs to.
    pub fn selected_area(&self) -> Option<String> {
        self.sidebar_items()
            .into_iter()
            .find(|i| i.selected)
            .map(|i| i.label)
    }

    /// The rectangles of the tiles of the Home grid, in
    /// [`HUB_TILES`] order, from the last drawn frame.
    pub fn tile_rects(&self) -> Vec<Rect> {
        (0..HUB_TILES.len())
            .filter_map(|i| self.rect_of(ids::at(ids::HOME_TILE_BASE, i)))
            .collect()
    }

    /// The first `count` addresses of key `key` (by index) for `script`
    /// on the receive or change chain, on the current network.
    pub fn addresses(
        &self,
        key: usize,
        script: ScriptType,
        change: bool,
        count: u32,
    ) -> Vec<String> {
        let Some(master) = self.keys.get(key).and_then(|k| k.master.as_ref()) else {
            return Vec::new();
        };
        let Ok(account) = master.account_xpub(script, 0) else {
            return Vec::new();
        };
        (0..count)
            .filter_map(|i| account.address(change, i).ok())
            .map(|a| alloc::format!("{a}"))
            .collect()
    }

    /// The rectangle of widget `id` in the last drawn frame, for
    /// computing tap positions in tests.
    pub fn rect_of(&self, id: Id) -> Option<Rect> {
        self.layout.as_ref()?.rect(id)
    }

    /// The item the focus ring is on (`docs/DESIGN.md` §4.15), or
    /// `None` while no key has moved focus on this screen.
    pub fn focused(&self) -> Option<Id> {
        self.ui.focused()
    }

    /// Every rectangle the last frame placed, in drawing order. Layout
    /// tests use it to assert that a screen does not move: revealing a
    /// secret must leave every other rectangle where it was (UX review
    /// 2026-09-07, finding 9).
    pub fn placed_rects(&self) -> Vec<Rect> {
        self.layout
            .as_ref()
            .map_or_else(Vec::new, |l| l.items.iter().map(|p| p.rect).collect())
    }

    /// How the chunked string `id` was laid out last frame. A line takes
    /// every group of four the width holds, and the whole string is on
    /// the screen; layout tests assert that.
    pub fn chunk_fit(&self, id: Id) -> Option<osk_ui::widgets::ChunkFit> {
        fn find(node: &Node, id: Id) -> Option<&Widget> {
            if let Some(w) = node.as_widget()
                && w.id() == Some(id)
                && matches!(w, Widget::ChunkedString { .. })
            {
                return Some(w);
            }
            node.children.iter().find_map(|c| find(c, id))
        }
        let rect = self.rect_of(id)?;
        let tree = self.build();
        let widget = find(&tree, id)?;
        osk_ui::widgets::chunk_fit(widget, &self.layout_ctx(), rect)
    }

    /// Every string the last frame drew, with the rectangle it was
    /// given, the space it needed there and the space it would need on
    /// one line. [`cut_texts`](Self::cut_texts) and
    /// [`wrapped_texts`](Self::wrapped_texts) read it.
    fn text_boxes(&self) -> Vec<(String, Rect, Size, Size)> {
        fn walk(
            node: &Node,
            layout: &Layout,
            ctx: &LayoutCtx,
            next: &mut usize,
            out: &mut Vec<(String, Rect, Size, Size)>,
        ) {
            let Some(placed) = layout.items.get(*next) else {
                return;
            };
            let rect = placed.rect;
            *next += 1;
            if let Some(widget) = node.as_widget()
                && let Widget::Text {
                    text, font, wrap, ..
                } = widget
            {
                // A line that does not wrap walks down the type ramp
                // before anything is cut, so what the reader sees is the
                // line at the size it was finally drawn at.
                let given = if *wrap {
                    widget.measure(ctx, Size::new(rect.w, i32::MAX))
                } else {
                    osk_ui::widgets::one_line_size(ctx, text, *font, rect.w)
                };
                let one_line = widget.measure(ctx, Size::new(i32::MAX, i32::MAX));
                out.push((text.clone(), rect, given, one_line));
            }
            for c in &node.children {
                walk(c, layout, ctx, next, out);
            }
        }
        let Some(layout) = self.layout.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut next = 0;
        walk(
            &self.build(),
            layout,
            &self.layout_ctx(),
            &mut next,
            &mut out,
        );
        out
    }

    /// Every string the last frame drew, with the rectangle it was
    /// given. A layout test reads it to put what a person sees against
    /// the box it is supposed to sit in — the words of a secret panel
    /// against the panel.
    pub fn text_rects(&self) -> Vec<(String, Rect)> {
        self.text_boxes()
            .into_iter()
            .map(|(text, rect, ..)| (text, rect))
            .collect()
    }

    /// Every string the last frame drew in a rectangle too small to
    /// hold it: the words a reader finds cut off, or a wrapped line the
    /// bottom of the box hides. Layout tests read it to assert that what
    /// a screen states is on it whole.
    pub fn cut_texts(&self) -> Vec<String> {
        self.text_boxes()
            .into_iter()
            .filter(|(_, rect, given, _)| given.w > rect.w || given.h > rect.h)
            .map(|(text, ..)| text)
            .collect()
    }

    /// Every string the last frame drew on more than one line. A test
    /// reads it to assert that a screen says something on one line, as
    /// the row it is drawn as does.
    pub fn wrapped_texts(&self) -> Vec<String> {
        self.text_boxes()
            .into_iter()
            .filter(|(_, _, given, one_line)| given.h > one_line.h)
            .map(|(text, ..)| text)
            .collect()
    }

    /// What the frame on screen does wrong that a person would see:
    /// strings or icons drawn over each other, data drawn outside the
    /// mono face, text cut at the side of its clip (`osk_ui::audit`).
    /// The frame is painted whole, as [`OpenSigner::clipped_texts`] does.
    pub fn audit(&mut self) -> Vec<String> {
        let Self {
            canvas,
            tree,
            layout,
            theme,
            ui,
            dirty,
            ..
        } = self;
        let (Some(canvas), Some(tree), Some(layout)) =
            (canvas.as_mut(), tree.as_ref(), layout.as_ref())
        else {
            return Vec::new();
        };
        canvas.record_ink(true);
        canvas.clear(theme.background);
        widgets::draw_tree(canvas, tree, layout, theme, ui);
        *dirty = false;
        let mut found = osk_ui::audit::check(canvas.ink(), canvas.scale().factor());
        found.extend(
            canvas
                .cut_texts()
                .iter()
                .map(|t| alloc::format!("cut: {t:?}")),
        );
        canvas.record_ink(false);
        found
    }

    /// Every string the current frame draws cut off by the side of its
    /// clip: a row label that runs under its chevron, a word past its
    /// cell's edge (`docs/PLANNING.md` §16.133 rule 3). The frame is
    /// painted whole first, so what is reported is the screen as a
    /// person sees it, not a strip a scroll redrew.
    pub fn clipped_texts(&mut self) -> Vec<String> {
        let Self {
            canvas,
            tree,
            layout,
            theme,
            ui,
            dirty,
            ..
        } = self;
        let (Some(canvas), Some(tree), Some(layout)) =
            (canvas.as_mut(), tree.as_ref(), layout.as_ref())
        else {
            return Vec::new();
        };
        canvas.clear(theme.background);
        widgets::draw_tree(canvas, tree, layout, theme, ui);
        *dirty = false;
        canvas.cut_texts().to_vec()
    }

    /// The chunked strings the last frame had to page, with the
    /// rectangle each was given. A secret panel must never hold one: its
    /// surface is the reveal target, so the pager would be under the
    /// finger (UX review 2026-09-07, finding 6).
    pub fn paged_chunks(&self) -> Vec<(Id, Rect)> {
        self.layout.as_ref().map_or_else(Vec::new, |l| {
            l.items
                .iter()
                .filter_map(|p| match p.hit {
                    Some(HitTarget::Pager { id, pages }) if pages > 1 => Some((id, p.rect)),
                    _ => None,
                })
                .collect()
        })
    }

    /// Widgets the last frame placed outside the region they are drawn
    /// in: a row below the fold, a card past the edge. Empty on a screen
    /// that fits, which is what every screen but a list is built to do
    /// (UX.md §5). Layout tests assert on this.
    pub fn overflow(&self) -> Vec<Rect> {
        self.layout.as_ref().map_or_else(Vec::new, |l| {
            l.items
                .iter()
                .filter(|p| {
                    !p.rect.is_empty()
                        && (p.rect.y < p.clip.y
                            || p.rect.bottom() > p.clip.bottom()
                            || p.rect.x < p.clip.x
                            || p.rect.right() > p.clip.right())
                })
                .map(|p| p.rect)
                .collect()
        })
    }

    /// `(content height, applied offset)` of scroll region `id` in the
    /// last drawn frame, for layout tests and review scripts.
    pub fn scroll_info(&self, id: Id) -> Option<(i32, i32)> {
        let placed = self.layout.as_ref()?.placed(id)?;
        let info = placed.scroll?;
        Some((info.content_h, info.offset))
    }

    /// Draws the current state over the whole panel. For tests and
    /// review scripts, which compare the panel a scroll left behind —
    /// pixels moved, one strip drawn — with the panel a full draw gives.
    pub fn redraw(&mut self) {
        self.render();
    }

    /// The layout context of the last drawn frame (scale and size class).
    pub fn layout_ctx(&self) -> LayoutCtx {
        self.ctx
            .unwrap_or_else(|| LayoutCtx::new(Scale::IDENTITY, SizeClass::Small))
    }

    /// Scrolls the current screen so that widget `id` is fully inside its
    /// clip, redraws, and returns its new rectangle. For tests and review
    /// scripts, which tap by id on every screen size; a widget that is not
    /// on the screen at all gives `None`.
    pub fn reveal(&mut self, id: Id) -> Option<Rect> {
        let placed = self.layout.as_ref()?.placed(id)?;
        let (rect, clip) = (placed.rect, placed.clip);
        let dy = if rect.bottom() > clip.bottom() {
            rect.bottom() - clip.bottom()
        } else if rect.y < clip.y {
            rect.y - clip.y
        } else {
            return Some(rect);
        };
        let offset = self.ui.scroll_offsets().get(ids::SCROLL);
        self.ui.set_scroll(ids::SCROLL, (offset + dy).max(0));
        self.render();
        self.layout.as_ref()?.rect(id)
    }

    /// The wording this session shows. A second language is another
    /// static of the same shape (`strings/en.rs`).
    pub fn strings(&self) -> &'static Strings {
        &EN
    }

    /// The display's size class.
    pub fn class(&self) -> SizeClass {
        self.ctx.map_or(SizeClass::Small, |c| c.class)
    }

    /// The whole display in dp, which is what a screen built from
    /// [`osk_ui::screens`] is drawn in: those screens cap their own
    /// column and draw their own `wide` sidebar, so they are given the
    /// window rather than the pane [`metrics`](Self::metrics) carves
    /// out of it for the older composites.
    fn window(&self) -> Metrics {
        let ctx = self.layout_ctx();
        let bounds = self
            .canvas
            .as_ref()
            .map_or(Rect::new(0, 0, 268, 358), Canvas::bounds);
        let factor = ctx.scale.factor();
        Metrics::new(
            bounds.w as f32 / factor,
            bounds.h as f32 / factor,
            ctx.class,
        )
        .with_insets(ctx.inset_top_dp, ctx.inset_bottom_dp)
    }

    /// Width in dp the rows of a words panel have: the pane the §5
    /// screens draw in, which is the window less the `wide` sidebar.
    fn words_pane_dp(&self) -> f32 {
        let m = self.window();
        if m.is_wide() {
            m.width_dp - osk_ui::tokens::SIDEBAR_WIDTH
        } else {
            m.width_dp
        }
    }

    /// Builds a screen from [`osk_ui::screens`] in the chrome this
    /// session gives it: the window, the areas the `wide` sidebar lists,
    /// the back chevron, and whether the sidebar is inert here.
    pub(crate) fn with_chrome(
        &self,
        back: Option<Id>,
        f: impl FnOnce(&Chrome<'_>) -> Node,
    ) -> Node {
        let m = self.window();
        let nav = self.sidebar_items();
        f(&Chrome {
            m: &m,
            nav: &nav,
            back,
            dimmed: self.sidebar_dimmed(),
            // §16.105: the trailing slot's info button, where this
            // screen has a Learn page. The eye wins the slot on the
            // screens that carry one.
            info: self.learn_target().map(|_| ids::INFO),
        })
    }

    /// Side in pixels of the square the Sign QR page gives its code: the
    /// body between the app bar and the mode row, capped where the QR
    /// widget caps itself. The module pitch that decides static against
    /// animated is measured in this square (UX review 2026-09-07, §2.8).
    fn qr_side_px(&self) -> i32 {
        // §4.9 draws the square at the class's side, so the pitch that
        // decides one code against the animated parts is measured there.
        let width = (self.window().width_dp - 2.0 * tokens::PAD).max(0.0);
        self.layout_ctx()
            .px(tokens::qr_side(self.class()).min(width))
    }

    /// Whether the content pane holds a capped column rather than a
    /// document: a wizard, the Sign wizard, the scanner, the lock screen
    /// and the terminal screens, whose buttons and keyboards no more span
    /// a 720 dp pane than they span a window.
    fn wizard_column(&self) -> bool {
        self.wizard.is_some()
            || self.selftest_failed()
            || match self.screen {
                Screen::Scan | Screen::Ended | Screen::NoSecureBoot => true,
                Screen::Sign => matches!(self.sign.stage(), Stage::Wizard(_)),
                _ => false,
            }
    }

    /// Whether the `wide` sidebar is drawn dimmed and without hit
    /// targets: the wizards, the Sign wizard, the scanner, the lock
    /// screen, the four hold-to-confirm screens and the terminal screens
    /// (`docs/PLANNING.md` §16.29). It is on every `Wide` screen either
    /// way, so nothing shifts sideways between two screens of one flow.
    fn sidebar_dimmed(&self) -> bool {
        self.wizard_column()
            || self.session.is_locked()
            || self.front_door()
            || matches!(
                self.screen,
                Screen::Forget(_)
                    | Screen::Keep(_)
                    | Screen::DuressPin
                    | Screen::OpenPassphrase(_)
                    | Screen::OpenChild(_)
                    | Screen::Bip85(_)
                    | Screen::Vanity(_)
                    | Screen::WipeAll
                    | Screen::WipeAndExit
            )
    }

    /// The sidebar's entries: the six areas of the launcher, with the
    /// current one selected. An area row carries the same id as the
    /// launcher's tile, because no screen ever draws both. A key's
    /// screens belong to Keys and a wallet's to Wallets, which is where
    /// their rows are.
    fn sidebar_items(&self) -> Vec<SidebarItem> {
        let s = self.strings();
        let area = match self.screen {
            Screen::Inspect | Screen::Tiers | Screen::Home => None,
            Screen::Scan => Some(TILE_SCAN),
            Screen::Keys
            | Screen::Created(_)
            | Screen::Add
            | Screen::PickKey(_)
            | Screen::KeyDetail(_)
            | Screen::BackupMenu(_)
            | Screen::OpenPassphrase(_)
            | Screen::OpenChild(_)
            | Screen::Bip85(_)
            | Screen::Vanity(_)
            | Screen::Opened(..)
            | Screen::Forget(_)
            | Screen::KeyExport(..)
            | Screen::Keep(_) => Some(TILE_KEYS),
            Screen::Wallets
            | Screen::AddWallet
            | Screen::WalletKeys(_)
            | Screen::Wallet(_)
            | Screen::WalletName(_)
            | Screen::Addresses(_)
            | Screen::Export(_)
            | Screen::SilentAddress(..)
            | Screen::SilentLabels(_)
            | Screen::SilentCheck(_)
            | Screen::SilentDns(_)
            | Screen::SilentSecret(_)
            | Screen::Sign
            | Screen::SignKeys(_)
            | Screen::SignMessage
            | Screen::Verify
            | Screen::CheckedMessage => Some(TILE_WALLETS),
            Screen::Explore
            | Screen::Tools
            | Screen::WordList
            | Screen::Word(_)
            | Screen::DicePassphrase
            | Screen::Tool(_)
            | Screen::ToolResult(_)
            | Screen::Decode
            | Screen::CompareTx
            | Screen::Lightning
            | Screen::Notes
            | Screen::NoteText
            | Screen::Note
            | Screen::OpenedSheet => Some(TILE_TOOLS),
            // The form, the passphrase and the file are steps of
            // whatever opened them, so they belong to that area; a
            // sheet is a wallet's.
            Screen::Sheet(_)
            | Screen::ExportForm
            | Screen::SealPass
            | Screen::Sealed
            | Screen::SealedQr => Some(TILE_WALLETS),
            Screen::Learn | Screen::LearnPage(_) | Screen::LearnTopic(..) => Some(TILE_LEARN),
            Screen::Settings | Screen::About | Screen::WipeAll | Screen::Wiped(_) => {
                Some(TILE_SETTINGS)
            }
            Screen::StartHere
            | Screen::DuressPin
            | Screen::KeptRemoved
            | Screen::WipeAndExit
            | Screen::Ended
            | Screen::NoSecureBoot => None,
        };
        HUB_TILES
            .iter()
            .enumerate()
            .map(|(i, icon)| SidebarItem {
                id: Some(ids::at(ids::HOME_TILE_BASE, i)),
                icon: *icon,
                label: String::from(hub_label(i, s)),
                selected: area == Some(i),
            })
            .collect()
    }

    // ----- state used by the views -----

    // ----- Tools > Lightning node key (§16.116) -----

    /// How many loaded keys ldk-node's derivation applies to: the ones
    /// with a 64-byte BIP-39 seed, which is the only thing ldk-node
    /// takes.
    fn lightning_keys(&self) -> usize {
        self.keys.iter().filter(|k| k.seed_len() == 64).count()
    }

    /// Opens the tool over the loaded key at `key`.
    fn open_lightning_key(&mut self, key: usize) {
        if self.keys.get(key).is_none_or(|k| k.seed_len() != 64) {
            return;
        }
        self.lightning = Some(Lightning::loaded(key));
        self.push(Screen::Lightning);
    }

    /// Opens the Load wizard's word entry as the tool's cipher-seed
    /// input: twenty-four words over the English list, which is the
    /// list LND publishes as its own. The words never become a key.
    fn start_aezeed_entry(&mut self) {
        let mut w = LoadWizard::new();
        w.set_count(osk_bip::aezeed::NUM_WORDS as u8);
        w.set_language(osk_bip::bip39::Language::English);
        w.set_free_last(true);
        w.go(Step::Words);
        self.wizard = Some(Wizard::Load(w));
        self.aezeed_entry = true;
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The word entry's twenty-four words, once they are all in.
    ///
    /// An aezeed carries no BIP-39 checksum — its own check is the
    /// CRC-32C inside the ciphertext — so the wizard's checksum result
    /// is never shown: the indices come here and the wizard is dropped.
    fn take_aezeed_words(&mut self) {
        if !self.aezeed_entry {
            return;
        }
        let Some(Wizard::Load(w)) = &self.wizard else {
            return;
        };
        if w.step() != Step::Checksum {
            return;
        }
        let indices: Vec<u16> = w.committed_indices().collect();
        if let Some(flow) = self.lightning.as_mut() {
            flow.set_words(&indices);
        }
        self.cancel_wizard();
    }

    /// Runs before the tool is drawn: a loaded key's node secret is
    /// derived once, and kept beside the screen.
    fn prepare_lightning(&mut self) {
        let network = self.network;
        let source = self.lightning.as_ref().map(lightning::Lightning::source);
        if let Some(lightning::Source::Loaded(i)) = source
            && self
                .lightning
                .as_ref()
                .is_some_and(|l| l.answer().is_none())
            && let Some(key) = self.keys.get(i)
        {
            let seed = key.seed(self.session.key(), |seed| {
                let mut bytes = [0u8; 64];
                let ok = seed.len() == 64;
                if ok {
                    bytes.copy_from_slice(seed);
                }
                (ok, bytes)
            });
            if let Some((true, mut bytes)) = seed {
                if let Some(flow) = self.lightning.as_mut() {
                    flow.resolve_loaded(&bytes, network);
                }
                bytes.zeroize();
            }
        }
    }

    /// The chevron inside the tool: the Secret screen goes back to the
    /// Result, the Result of a typed cipher seed to its passphrase, and
    /// anything else leaves.
    fn lightning_back(&mut self) -> bool {
        let Some(flow) = self.lightning.as_mut() else {
            return false;
        };
        match flow.step() {
            lightning::Step::Secret(_) => {
                flow.go(lightning::Step::Result);
                true
            }
            lightning::Step::Result if flow.source() == lightning::Source::Aezeed => {
                flow.go(lightning::Step::Passphrase);
                true
            }
            _ => false,
        }
    }

    /// A tap on one of the tool's screens.
    fn tap_lightning(&mut self, id: Id) {
        if id == ids::LIGHTNING_DONE {
            self.go_home();
            return;
        }
        if id == ids::LIGHTNING_NODE_KEY {
            let title = String::from(self.strings().lightning_node_key);
            if let Some(lightning::Answer::Node(node)) =
                self.lightning.as_ref().and_then(Lightning::answer)
            {
                let value = text::hex(&node.public);
                self.open_compare(&title, value);
            }
            return;
        }
        let Some(flow) = self.lightning.as_mut() else {
            return;
        };
        if id == ids::LIGHTNING_SECRET || id == ids::LIGHTNING_NODE_SECRET {
            flow.go(lightning::Step::Secret(lightning::SecretKind::NodeKey));
        } else if id == ids::LIGHTNING_ENTROPY {
            flow.go(lightning::Step::Secret(lightning::SecretKind::Entropy));
        }
    }

    // ----- the vanity address grinder (`docs/PLANNING.md` §16.117) -----

    /// Whether `dial` can be turned over the key at `key`.
    ///
    /// The passphrase dial needs the key's words, and it needs the key
    /// not to carry a passphrase already: this device never keeps the
    /// passphrase a key was opened with, so there is nothing to append
    /// a counter to, and grinding the words alone would name a key that
    /// is not this one's child.
    pub(crate) fn vanity_dial_available(&self, key: usize, dial: vanity::Dial) -> bool {
        let Some(k) = self.keys.get(key) else {
            return false;
        };
        match dial {
            vanity::Dial::Passphrase => k.has_mnemonic() && !k.has_passphrase,
            vanity::Dial::Account => k.master.is_some(),
        }
    }

    /// Whether this device's hardware makes a grind slow: Tier A is a
    /// phone or a desktop doing the work in software, and a passphrase
    /// candidate there costs a whole PBKDF2.
    pub(crate) fn tier_is_a(&self) -> bool {
        self.tier == AssuranceTier::A
    }

    /// Why the passphrase dial is dimmed on this key.
    pub(crate) fn vanity_dial_reason(&self, key: usize) -> &'static str {
        let s = self.strings();
        match self.keys.get(key) {
            Some(k) if k.has_passphrase => s.vanity_has_passphrase,
            _ => s.vanity_no_words,
        }
    }

    /// The dial the Choice opens checked: the first one the key can
    /// turn.
    fn vanity_first_dial(&self, key: usize) -> usize {
        vanity::Dial::ALL
            .iter()
            .position(|d| self.vanity_dial_available(key, *d))
            .unwrap_or(0)
    }

    /// The Choice's answer: the grind begins at its script type.
    fn start_vanity(&mut self, key: usize, dial: vanity::Dial) {
        if !self.vanity_dial_available(key, dial) {
            return;
        }
        self.vanity = Some(vanity::Grind::new(
            dial,
            ScriptType::NativeSegwit,
            self.network,
        ));
        self.push(Screen::Vanity(key));
    }

    /// The chevron inside the grinder: each step goes back to the one
    /// before it, and the first step leaves. A run that is stopped this
    /// way is discarded with everything it found.
    fn vanity_back(&mut self) -> bool {
        let network = self.network;
        let Some(g) = self.vanity.as_mut() else {
            return false;
        };
        match g.step {
            vanity::Step::Script => false,
            vanity::Step::Prefix => {
                g.step = vanity::Step::Script;
                true
            }
            vanity::Step::Secret => {
                g.step = vanity::Step::Found;
                true
            }
            vanity::Step::Running | vanity::Step::Found => {
                g.step = vanity::Step::Prefix;
                g.find = None;
                g.prefix = vanity::Prefix::fixed(g.script, network);
                true
            }
        }
    }

    /// A tap on one of the grinder's screens.
    fn tap_vanity(&mut self, id: Id, key: usize) {
        if id == ids::VANITY_ADDRESS {
            let title = String::from(self.strings().row_address);
            if let Some(address) = self
                .vanity
                .as_ref()
                .and_then(|g| g.find.as_ref())
                .map(|f| String::from(f.address.as_str()))
            {
                self.open_compare(&title, address);
            }
            return;
        }
        if id == ids::VANITY_USE {
            self.vanity_use(key);
            return;
        }
        let network = self.network;
        let Some(g) = self.vanity.as_mut() else {
            return;
        };
        if let Some(i) = ids::index_in(id, ids::VANITY_SCRIPT_BASE, ScriptType::ALL.len()) {
            g.set_script(ScriptType::ALL[i], network);
        } else if id == ids::VANITY_SCRIPT_CONTINUE {
            g.step = vanity::Step::Prefix;
            self.ui.set_scroll(ids::SCROLL, 0);
        } else if id == ids::VANITY_STOP {
            g.step = vanity::Step::Prefix;
        } else if id == ids::VANITY_SHOW && g.find.is_some() {
            g.step = vanity::Step::Secret;
        } else if id == ids::VANITY_DONE {
            g.step = vanity::Step::Found;
        }
    }

    /// A character of the prefix. The keyboard offers only characters
    /// an address of this kind can carry, so a key that lands here is
    /// one the encoding allows; ✓ runs the grind.
    fn vanity_typed(&mut self, input: KeyInput) {
        let network = self.network;
        let now = self.now_ms;
        let Some(g) = self.vanity.as_mut() else {
            return;
        };
        if g.step != vanity::Step::Prefix {
            return;
        }
        let script = g.script;
        match input {
            KeyInput::Char(c) => {
                g.prefix.push(c, script, network);
            }
            KeyInput::Backspace => g.prefix.pop(script, network),
            KeyInput::Done => {
                if g.prefix.has_free(script, network) {
                    g.start(now);
                    self.ui.set_scroll(ids::SCROLL, 0);
                }
            }
            KeyInput::Shift | KeyInput::Symbols => {}
        }
    }

    /// One tick of a running grind: a budget of candidates, and what
    /// they took. `true` when the screen has something new to draw.
    fn vanity_tick(&mut self) -> bool {
        let Screen::Vanity(key) = self.screen else {
            return false;
        };
        let Some(g) = self.vanity.as_ref() else {
            return false;
        };
        if g.step != vanity::Step::Running || g.find.is_some() || g.exhausted {
            return false;
        }
        let (dial, script, cursor, budget) = (g.dial, g.script, g.cursor, g.budget);
        let prefix = String::from(g.prefix.as_str());
        let network = self.network;
        let now = self.now_ms;
        let outcome = match dial {
            vanity::Dial::Passphrase => self.keys.get(key).and_then(|k| {
                k.mnemonic(self.session.key(), |m| {
                    osk_bip::vanity::grind(
                        &osk_bip::vanity::Key::Words(m),
                        &osk_bip::vanity::Method::Passphrase { base: b"" },
                        script,
                        &prefix,
                        network,
                        cursor,
                        budget,
                    )
                })
            }),
            vanity::Dial::Account => {
                self.keys
                    .get(key)
                    .and_then(|k| k.master.as_ref())
                    .map(|master| {
                        osk_bip::vanity::grind(
                            &osk_bip::vanity::Key::Master(master),
                            &osk_bip::vanity::Method::Account,
                            script,
                            &prefix,
                            network,
                            cursor,
                            budget,
                        )
                    })
            }
        };
        let Some(outcome) = outcome else {
            return false;
        };
        let Some(g) = self.vanity.as_mut() else {
            return false;
        };
        g.ticked(outcome.tested, now);
        if outcome.find.is_some() {
            g.find = outcome.find;
            g.step = vanity::Step::Found;
        } else if g.exhausted {
            g.step = vanity::Step::Found;
        }
        true
    }

    /// "Use it": the passphrase dial opens the key its counter names,
    /// exactly as "Open with passphrase" does, and the account dial
    /// adds the single-sig wallet at the account it found.
    fn vanity_use(&mut self, key: usize) {
        let Some(g) = self.vanity.take() else {
            return;
        };
        let Some(find) = g.find.as_ref() else {
            self.vanity = Some(g);
            return;
        };
        match g.dial {
            vanity::Dial::Passphrase => {
                let verified = self.keys.get(key).is_some_and(|k| k.backup_verified);
                let network = self.network;
                let opened = self.key_mnemonic(key).and_then(|mnemonic| {
                    let seed = mnemonic.to_seed(find.suffix.as_bytes()).ok()?;
                    let seed = SeedBytes::new(seed.expose()).map(Secret::new)?;
                    let master = MasterKey::from_seed_bytes(&seed, network);
                    Some(LoadedKey::new(seed, master, Some(mnemonic), true, verified).derived())
                });
                let Some(opened) = opened else {
                    return;
                };
                self.open_derived(opened, OpenedFrom::Passphrase);
            }
            vanity::Dial::Account => {
                let script = g.script;
                let account = find.account;
                let key_text = self
                    .keys
                    .get(key)
                    .and_then(|k| k.master.as_ref())
                    .and_then(|master| master.account_xpub(script, account).ok())
                    .map(|a| build::account_key(a.master_fingerprint(), a.path(), a.xpub()));
                let Some(key_text) = key_text else {
                    return;
                };
                let template = match script {
                    ScriptType::Legacy => "pkh(@0/**)",
                    ScriptType::NestedSegwit => "sh(wpkh(@0/**))",
                    ScriptType::NativeSegwit => "wpkh(@0/**)",
                    ScriptType::Taproot => "tr(@0/**)",
                };
                let Ok(policy) =
                    osk_bip::policy::WalletPolicy::from_parts(template, &[key_text.as_str()])
                else {
                    return;
                };
                self.add_built_wallet(policy);
            }
        }
    }

    // ----- explore -----

    /// The master key Explore derives from, if any.
    fn explore_master(&self) -> Option<&MasterKey> {
        match self.explore.source() {
            Source::None => None,
            Source::Loaded(i) => self.keys.get(i)?.master.as_ref(),
            Source::Typed => self.explore.typed_master(),
        }
    }

    /// Runs `f` on the words Explore derives from: the loaded key's,
    /// unsealed for the call, or the typed ones.
    pub(crate) fn explore_with_mnemonic<R>(&self, f: impl FnOnce(&Mnemonic) -> R) -> Option<R> {
        match self.explore.source() {
            Source::None => None,
            Source::Loaded(i) => self.keys.get(i)?.mnemonic(self.session.key(), f),
            Source::Typed => self.explore.typed_mnemonic().map(|m| f(&m)),
        }
    }

    /// Runs `f` on the 64-byte seed Explore derives from: the loaded
    /// key's, unsealed for the call, or PBKDF2 of the typed words and
    /// passphrase, computed now and dropped after.
    pub(crate) fn explore_with_seed<R>(&self, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        match self.explore.source() {
            Source::None => None,
            Source::Loaded(i) => self.keys.get(i)?.seed(self.session.key(), f),
            Source::Typed => {
                let m = self.explore.typed_mnemonic()?;
                let seed = m.to_seed(self.explore.passphrase_bytes()).ok()?;
                Some(f(seed.expose().as_slice()))
            }
        }
    }

    pub(crate) fn explore_cached(&self) -> Option<&ExploreCache> {
        self.explore_cache.as_ref()
    }

    /// Derives what the Explore path and encodings sections show, if the
    /// cache does not hold it already.
    fn prepare_explore(&mut self) {
        // Typed words build a master key of their own, outside the
        // loaded keys the session blinds on the way in, so the blind is
        // handed to the workspace here: this runs on every frame the
        // screen is up, so a rotation reaches it too (security review
        // M1).
        self.explore.set_blind(self.session.secp_blind());
        let Some(master) = self.explore_master() else {
            self.explore_cache = None;
            return;
        };
        let fingerprint = master.fingerprint();
        let network = self.network;
        let path = self.explore.applied_path().clone();
        if self
            .explore_cache
            .as_ref()
            .is_some_and(|c| c.fingerprint == fingerprint && c.network == network && c.path == path)
        {
            return;
        }
        let master_xpub = master.xpub();
        let children = path.as_ref();
        let mut levels = Vec::with_capacity(children.len());
        let mut leaf = master_xpub;
        for depth in 1..=children.len() {
            let sub = DerivationPath::from(&children[..depth]);
            let derived = master.derive(&sub);
            leaf = derived.to_xpub();
            levels.push(ExploreLevel {
                xpub: alloc::format!("{leaf}"),
            });
        }
        let implied = explore::implied_script(&path);
        let addresses = ScriptType::ALL
            .iter()
            .copied()
            .filter(|s| implied.is_none_or(|i| i == *s))
            .map(|s| (s, leaf_address(&leaf, s, network)))
            .collect();
        let account_depth = children.len().min(3);
        let account_path = DerivationPath::from(&children[..account_depth]);
        let account_xpub = if account_depth == children.len() {
            leaf
        } else {
            master.derive(&account_path).to_xpub()
        };
        let account_slip132 = implied
            .filter(|s| matches!(s, ScriptType::NestedSegwit | ScriptType::NativeSegwit))
            .map(|s| (s, osk_bip::slip132::encode_xpub(&account_xpub, s)));
        self.explore_cache = Some(ExploreCache {
            fingerprint,
            network,
            path,
            master_xpub: alloc::format!("{master_xpub}"),
            levels,
            addresses,
            account_xpub: alloc::format!("{account_xpub}"),
            account_slip132,
        });
    }

    /// Opens the Load wizard's word entry as Explore's typed input
    /// (UX.md §7.5): count, language, words and the checksum diagnosis,
    /// after which the indices come here and no key is added.
    fn start_explore_entry(&mut self) {
        let mut w = LoadWizard::new();
        w.go(Step::Count);
        self.wizard = Some(Wizard::Load(w));
        self.explore_entry = true;
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// How many words the open wizard's key has, for the record its
    /// confirm step states.
    pub(crate) fn wizard_words(&self) -> usize {
        match &self.wizard {
            Some(Wizard::Load(w)) => usize::from(w.count()),
            Some(Wizard::Create(w)) => usize::from(w.count()),
            _ => 0,
        }
    }

    /// What a loaded key's fingerprint stands for in a typed policy: the
    /// account key expression the miniscript compiler substitutes for
    /// it, so that a policy can be typed `pk(73c5da0a)` and still name a
    /// real key.
    ///
    /// The account follows the wrapper being compiled: BIP-48's p2wsh
    /// account for a `wsh` policy, BIP-86's for a `tr` one. Both carry
    /// the multipath a wallet needs, so a policy of loaded keys compiles
    /// into a descriptor that loads as a wallet.
    /// One loaded key's account extended public key, at the script type
    /// the device's own default uses: what Convert key's "Use a loaded
    /// key" row feeds the field.
    fn account_xpub_of(&self, key: usize) -> Option<String> {
        let master = self.keys.get(key)?.master.as_ref()?;
        let account = master.account_xpub(ScriptType::NativeSegwit, 0).ok()?;
        Some(alloc::format!("{}", account.xpub()))
    }

    pub(crate) fn policy_key_expressions(&self) -> Vec<(String, String)> {
        let script = self.calc.script();
        self.keys
            .iter()
            .filter_map(|key| {
                let master = key.master.as_ref()?;
                let text = match script {
                    osk_bip::compile::PolicyScript::Segwit => {
                        let a = master
                            .multisig_account_xpub(
                                osk_bip::keys::MultisigScriptType::NativeSegwit,
                                0,
                            )
                            .ok()?;
                        build::account_key(a.master_fingerprint(), a.path(), a.xpub())
                    }
                    osk_bip::compile::PolicyScript::Taproot => {
                        let a = master.account_xpub(ScriptType::Taproot, 0).ok()?;
                        build::account_key(a.master_fingerprint(), a.path(), a.xpub())
                    }
                };
                Some((
                    text::fingerprint_hex(key.fingerprint),
                    alloc::format!("{text}{}", osk_bip::policy::TEMPLATE_SUFFIX),
                ))
            })
            .collect()
    }

    fn cached_account(&self, fp: Fingerprint, script: ScriptType) -> Option<&AccountXpub> {
        self.cache
            .iter()
            .find(|c| c.fingerprint == fp && c.script == script)
            .map(|c| &c.account)
    }

    fn cached_addresses(&self, fp: Fingerprint, script: ScriptType, change: bool) -> &[String] {
        self.cache
            .iter()
            .find(|c| c.fingerprint == fp && c.script == script)
            .map_or(&[], |c| if change { &c.change } else { &c.receive })
    }

    /// The account xpub of `key` for `script`, from the cache, deriving
    /// and caching it on first use.
    fn account(&mut self, key: usize, script: ScriptType) -> Option<AccountXpub> {
        let k = self.keys.get(key)?;
        let fp = k.fingerprint;
        if let Some(a) = self.cached_account(fp, script) {
            return Some(a.clone());
        }
        let account = k.master.as_ref()?.account_xpub(script, 0).ok()?;
        self.cache.push(CachedAccount {
            fingerprint: fp,
            script,
            account: account.clone(),
            receive: Vec::new(),
            change: Vec::new(),
        });
        Some(account)
    }

    /// Every loaded key as the PSBT inspector sees it: fingerprint, the
    /// four single-signature accounts and BIP-48's two multisig accounts
    /// on the current network.
    ///
    /// The multisig accounts are derived here rather than cached: this
    /// runs once per PSBT, not per frame.
    /// The loaded keys that are members of some FROST group, as the
    /// inspector sees them: one public share each, which is what names a
    /// participant on a threshold input (§16.104 rule 3).
    fn share_refs(&self) -> Vec<osk_psbt::ShareRef> {
        self.keys
            .iter()
            .filter_map(|k| k.share)
            .map(|pubshare| osk_psbt::ShareRef { pubshare })
            .collect()
    }

    fn key_refs(&mut self) -> Vec<KeyRef> {
        let mut refs = Vec::with_capacity(self.keys.len());
        for i in 0..self.keys.len() {
            let accounts: Vec<AccountXpub> = ScriptType::ALL
                .iter()
                .filter_map(|s| self.account(i, *s))
                .collect();
            let multisig: Vec<MultisigAccountXpub> = self.keys[i]
                .master
                .as_ref()
                .map(|m| {
                    MultisigScriptType::ALL
                        .iter()
                        .filter_map(|s| m.multisig_account_xpub(*s, 0).ok())
                        .collect()
                })
                .unwrap_or_default();
            refs.push(KeyRef::new(self.keys[i].fingerprint, accounts, multisig));
        }
        refs
    }

    /// The addresses of the wallet whose list is open, as far as they
    /// have been derived.
    pub(crate) fn wallet_addresses(&self, wallet: usize, change: bool) -> &[String] {
        match &self.wallet_addresses {
            Some(a) if a.wallet == wallet => {
                if change {
                    &a.change
                } else {
                    &a.receive
                }
            }
            _ => &[],
        }
    }

    /// Derives what the wallet's address list is about to show. A
    /// multisig address is public data throughout: the policy's keys and
    /// the script they build.
    fn prepare_wallet_addresses(&mut self, wallet: usize) {
        let Some(policy) = self.wallets.get(wallet).cloned() else {
            return;
        };
        if self
            .wallet_addresses
            .as_ref()
            .is_none_or(|a| a.wallet != wallet)
        {
            self.wallet_addresses = Some(WalletAddresses {
                wallet,
                receive: Vec::new(),
                change: Vec::new(),
            });
        }
        let (network, change, wanted) = (self.network, self.detail.change, self.detail.shown);
        let entry = self.wallet_addresses.as_mut().expect("just inserted");
        let list = if change {
            &mut entry.change
        } else {
            &mut entry.receive
        };
        while list.len() < wanted {
            match policy.address_at(network, change, list.len() as u32) {
                Ok(address) => list.push(alloc::format!("{address}")),
                Err(_) => break,
            }
        }
    }

    /// The master Explore's typed words build, and its fingerprint,
    /// which is what the address cache is keyed by.
    pub(crate) fn typed_fingerprint(&self) -> Option<Fingerprint> {
        self.explore.typed_master().map(MasterKey::fingerprint)
    }

    /// Derives the addresses of Explore's typed words, the way a loaded
    /// key's are derived.
    fn prepare_typed_addresses(&mut self) {
        let Some(master) = self.explore.typed_master() else {
            return;
        };
        let fp = master.fingerprint();
        let (script, change, index) = (
            self.detail.script,
            self.detail.change,
            self.detail.shown.saturating_sub(1) as u32,
        );
        if !self
            .cache
            .iter()
            .any(|c| c.fingerprint == fp && c.script == script)
        {
            let Ok(account) = master.account_xpub(script, 0) else {
                return;
            };
            self.cache.push(CachedAccount {
                fingerprint: fp,
                script,
                account,
                receive: Vec::new(),
                change: Vec::new(),
            });
        }
        self.fill_addresses(fp, script, change, index);
    }

    /// Derives what the key detail screen is about to show, if it is not
    /// cached yet.
    fn prepare_detail(&mut self, key: usize) {
        let Some(k) = self.keys.get(key) else {
            return;
        };
        let (fp, script, change, index) = (
            k.fingerprint,
            self.detail.script,
            self.detail.change,
            self.detail.shown.saturating_sub(1) as u32,
        );
        if !self
            .cache
            .iter()
            .any(|c| c.fingerprint == fp && c.script == script)
        {
            let Some(Ok(account)) = k.master.as_ref().map(|m| m.account_xpub(script, 0)) else {
                return;
            };
            self.cache.push(CachedAccount {
                fingerprint: fp,
                script,
                account,
                receive: Vec::new(),
                change: Vec::new(),
            });
        }
        self.fill_addresses(fp, script, change, index);
    }

    /// Derives addresses of the cached account up to `index`.
    fn fill_addresses(&mut self, fp: Fingerprint, script: ScriptType, change: bool, index: u32) {
        let Some(entry) = self
            .cache
            .iter_mut()
            .find(|c| c.fingerprint == fp && c.script == script)
        else {
            return;
        };
        let list = if change {
            &mut entry.change
        } else {
            &mut entry.receive
        };
        while (list.len() as u32) <= index {
            match entry.account.address(change, list.len() as u32) {
                Ok(a) => list.push(alloc::format!("{a}")),
                Err(_) => break,
            }
        }
    }

    // ----- navigation -----

    fn push(&mut self, screen: Screen) {
        self.stack.push(self.screen);
        self.screen = screen;
        self.entered();
    }

    /// A screen was entered or returned to. The Sign flow's data lives only
    /// while its screen is on; any screen change drops it, and leaving the
    /// scanner turns the camera off.
    fn entered(&mut self) {
        // The first run ends when the address list it opened is left.
        if self.first_run_addresses && !matches!(self.screen, Screen::Addresses(_)) {
            self.finish_first_run();
        }
        self.ui.set_scroll(ids::SCROLL, 0);
        // §16.120: ids repeat from screen to screen, so the pressed look
        // a release left behind never crosses a screen change.
        self.ui.clear_linger();
        self.reveal_until = None;
        self.leave_deadline = None;
        self.overlay = None;
        self.open_error = None;
        // A wallet's review sends a person to Add a key and the key made
        // there comes back to it, checked against the member the row
        // named (§16.104 rule 6). Leaving that flow for one of the three
        // lists ends it: from Keys no wallet asked, and a key is a key.
        if matches!(self.screen, Screen::Home | Screen::Keys | Screen::Wallets) {
            self.add_key_return = None;
            self.key_detour = None;
        }
        // The account a key's Export derived belongs to that screen.
        if !matches!(self.screen, Screen::KeyExport(..)) {
            self.account_view = None;
        }
        self.sign.clear();
        self.leave_scan();
        if self.screen != Screen::SignMessage {
            self.message = None;
        }
        if self.screen != Screen::CheckedMessage {
            self.check = None;
        }
        self.wallet_addresses = None;
        match self.screen {
            Screen::Addresses(_) => self.detail = DetailState::default(),
            Screen::Export(owner) => {
                self.export = ExportState::default();
                // §16.113: a silent payments wallet is offered none of
                // the formats every other wallet has, so its export
                // opens on the first one it does have.
                if let WalletRef::Policy(w) = owner
                    && self.silent_of(w).is_some()
                {
                    self.export.format = ExportFormat::ALL
                        .iter()
                        .position(|f| *f == ExportFormat::SilentScan)
                        .unwrap_or(0);
                }
            }
            // A key's account export opens on the account key, with
            // nothing typed for a record.
            Screen::KeyExport(..) => {
                self.export = ExportState {
                    format: ExportFormat::xpub_index(),
                    ..ExportState::default()
                };
                self.bsms = BsmsEntry::default();
            }
            Screen::Verify => self.verify = VerifyState::new(),
            Screen::Explore if self.explore.source() == Source::None && !self.keys.is_empty() => {
                self.explore.use_loaded(0);
            }
            _ => {}
        }
        if self.screen != Screen::Inspect {
            self.inspect = None;
        }
        // The kept-secret pad and the blob it was working on belong to
        // the screens that show them. The stored key's own pad is not a
        // screen that is entered: it is what shows while no key is
        // loaded (`front_door`), so what it holds is cleared by its own
        // exchanges and by a wipe.
        if !matches!(self.screen, Screen::Keep(_) | Screen::DuressPin) && !self.front_door() {
            self.keep.clear();
        }
        if !matches!(self.screen, Screen::Keep(_)) {
            self.keep_offer = false;
        }
        // The passphrase typed to open a key and the child being chosen
        // belong to their own screens; leaving either zeroizes it.
        if !matches!(self.screen, Screen::OpenPassphrase(_)) {
            self.open_finish = None;
            self.mask_deadline = None;
            self.build_parked = None;
        }
        if !matches!(self.screen, Screen::OpenChild(_)) {
            self.child = None;
        }
        // The application being derived belongs to its own screen:
        // leaving it drops the parameters and the value with them
        // (§16.114).
        if !matches!(self.screen, Screen::Bip85(_)) {
            self.bip85 = None;
        }
        // The node key tool belongs to its own screen: leaving it
        // zeroizes the words, the passphrase, the entropy and the node
        // private key (§16.116).
        if !matches!(self.screen, Screen::Lightning) {
            self.lightning = None;
            self.aezeed_entry = false;
        }
        // The grind belongs to its own screens: leaving them discards
        // the find, the counter it was at and the passphrase it named
        // (§16.117).
        if !matches!(self.screen, Screen::Vanity(_)) {
            self.vanity = None;
        }
        // The address list over typed words is one of Explore's own
        // screens, so the words live while it is up.
        if self.screen != Screen::Explore && self.screen != Screen::Addresses(WalletRef::Typed) {
            // Dropping the workspace zeroizes any typed words, and the
            // addresses they derived go with them.
            if let Some(fp) = self.typed_fingerprint() {
                self.cache.retain(|c| c.fingerprint != fp);
            }
            self.explore = Explore::new(self.network);
            self.explore_entry = false;
            self.explore_cache = None;
        }
        // The word list's search and the dice tool's rolls belong to
        // their own screens; one word's Record is part of the word list,
        // so it keeps the language the search chose. Dropping the dice
        // tool zeroizes the rolls, which is what makes leaving the
        // passphrase leave nothing behind.
        if !matches!(self.screen, Screen::WordList | Screen::Word(_)) {
            self.word_list = WordList::new();
        }
        if self.screen != Screen::DicePassphrase {
            self.dice = DicePassphrase::new();
        }
        // A calculator's field belongs to the calculator: the answer
        // screen is part of the same tool and keeps it, and anything
        // else empties it. The decoded transaction goes the same way.
        if !matches!(self.screen, Screen::Tool(_) | Screen::ToolResult(_)) {
            self.calc = Calculator::new();
        }
        if self.screen != Screen::Decode {
            self.decode.clear();
        }
        // Compare transactions keeps the first transaction only while
        // the tool is open; the scanner it opens for the second is part
        // of the tool.
        if !matches!(self.screen, Screen::CompareTx | Screen::Scan) {
            self.compare_tx.clear();
        }
    }

    /// The stack under a wallet's Keys review, for a key added from one
    /// of its not-loaded rows (§16.104 rule 6): the review is where the
    /// person was, so it is where they come back to.
    fn return_stack(&mut self) -> Option<ReturnTo> {
        let r = self.add_key_return.take()?;
        match r.to {
            ReturnTo::Wallet(w) if w < self.wallets.len() => Some(ReturnTo::Wallet(w)),
            ReturnTo::Wallet(_) => None,
            ReturnTo::Transaction => Some(ReturnTo::Transaction),
        }
    }

    /// The member a wallet's Keys review named on the row Add a key was
    /// opened from, while that flow is running.
    fn asked_member(&self) -> Option<Member> {
        self.add_key_return.as_ref()?.member
    }

    /// Whether a key derived here is the member the wallet asked for
    /// (§16.104 rule 6). A key made from a wallet's not-loaded row is
    /// that wallet's key or it is nothing: the mismatch is stated and
    /// the key is not kept. From Keys directly, no wallet asked and a
    /// key is a key.
    fn wrong_key(&mut self, key: &LoadedKey, template: &'static str) -> bool {
        let Some(member) = self.asked_member() else {
            return false;
        };
        if member.matches(key) {
            return false;
        }
        let s = self.strings();
        let name = |fp: Option<Fingerprint>| {
            fp.map_or_else(|| String::from(s.value_none), text::fingerprint_hex)
        };
        let gave = name(member.fingerprint_of(key));
        let asked = name(member.fingerprint());
        self.open_error = Some(strings::fill(template, &[&gave, &asked]));
        true
    }

    /// Where a flow that added a key ends: the wallet's Keys review it
    /// was started from, or Home.
    fn after_key_added(&mut self) {
        match self.return_stack() {
            Some(ReturnTo::Wallet(w)) => {
                self.stack = alloc::vec![
                    Screen::Home,
                    Screen::Wallets,
                    Screen::Wallet(WalletRef::Policy(w))
                ];
                self.screen = Screen::WalletKeys(w);
                self.entered();
            }
            Some(ReturnTo::Transaction) => self.return_to_transaction(),
            None => self.go_home(),
        }
    }

    /// The transaction's Keys review, for a key added from one of its
    /// not-loaded rows (§16.110 rule 3). The flow was taken out whole
    /// before Add a key ran, because every screen change clears it; it
    /// goes back inspected again, with the key that just arrived, so the
    /// review's Continue comes alive and the key context names the key.
    fn return_to_transaction(&mut self) {
        let Some(reading) = self.key_detour.as_ref().map(|d| d.reading) else {
            self.go_home();
            return;
        };
        let under = if reading {
            Screen::Decode
        } else {
            Screen::Sign
        };
        self.stack = alloc::vec![Screen::Home, under];
        self.screen = Screen::SignKeys(reading);
        self.entered();
        self.restore_transaction();
    }

    /// Checks the signatures a loaded transaction carries against the
    /// nonce rules the keys this device holds could have made them
    /// with (`docs/PLANNING.md` §16.111 rule 3). Called after every
    /// load, read and re-inspection, because that check is the one the
    /// inspection cannot make: it has no key.
    fn check_transaction_signatures(&mut self, reading: bool) {
        let OpenSigner {
            sign, decode, keys, ..
        } = self;
        let masters: Vec<&MasterKey> = keys.iter().filter_map(|k| k.master.as_ref()).collect();
        let flow = if reading { decode } else { sign };
        flow.check(&masters);
    }

    /// Puts the transaction back where it was and inspects it again with
    /// the keys there are now. Called after the screen has settled,
    /// because every screen change clears the flow.
    fn restore_transaction(&mut self) {
        let Some(detour) = self.key_detour.take() else {
            return;
        };
        let refs = self.key_refs();
        let shares = self.share_refs();
        let wallets = self.wallets.clone();
        let network = self.network;
        let view = self.musig_session.as_ref().map(|s| s.view());
        let mut flow = detour.flow;
        flow.reinspect(refs, wallets, shares, network, view.as_ref());
        if detour.reading {
            self.decode = flow;
        } else {
            self.sign = flow;
        }
        self.check_transaction_signatures(detour.reading);
    }

    /// The transaction's Keys review, opened from the Sign review's key
    /// context row. It goes on the stack the way a Learn page does: the
    /// flow lives beside the stack and a `push` would clear it.
    fn open_transaction_keys(&mut self, reading: bool) {
        self.stack.push(self.screen);
        self.screen = Screen::SignKeys(reading);
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// §16.105: the info button opens its page over the screen it was
    /// tapped on. The page goes on the stack; the wizard, the flow and
    /// everything else beside the stack is left where it is, which is
    /// what makes the chevron a return rather than a cancel.
    fn open_learn_topic(&mut self, target: learn_map::LearnTarget) {
        let page = target.topic.index(self.strings());
        self.stack.push(self.screen);
        self.screen = Screen::LearnTopic(page, target.section);
        self.ui.set_scroll(ids::SCROLL, 0);
        if target.section.is_some() {
            self.scroll_into_view = Some((ids::LEARN_HEADING, Anchor::Top));
        }
    }

    fn go_home(&mut self) {
        self.stack.clear();
        self.screen = Screen::Home;
        self.entered();
    }

    /// One step toward Home: the previous wizard step, or the previous
    /// screen.
    fn back(&mut self) {
        // The session is over; there is nowhere to go back to.
        if self.screen == Screen::Ended {
            return;
        }
        // §16.105: a page opened from the info button leaves by the same
        // chevron, and what it was opened over is still there.
        if matches!(self.screen, Screen::LearnTopic(..)) {
            self.screen = self.stack.pop().unwrap_or(Screen::Home);
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        // The same for the transaction's Keys review: the transaction
        // is beside the stack and popping it the ordinary way would
        // clear it (§16.110 rule 3).
        if matches!(self.screen, Screen::SignKeys(_)) {
            self.screen = self.stack.pop().unwrap_or(Screen::Home);
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        // The first run's document is what Home draws until it is left:
        // the chevron ends the first run and Home becomes its list.
        if self.start_here_shown() {
            self.finish_first_run();
            self.leave_deadline = None;
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        // A Choice or a Compare sits over the screen that opened it, so
        // the chevron closes it before it leaves that screen (§4.1).
        if let Some(o) = self.overlay.take() {
            // Leaving the file list without choosing a file is choosing
            // no file: the screen that asked stops waiting.
            if matches!(o, Overlay::Files(_)) {
                self.cancel_file_request();
            }
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        // The scanner opened for a cosigner is a screen over the
        // builder: the chevron closes its own steps, then the scanner,
        // and the builder is where it was.
        if self.building_scan() {
            if let Some(s) = self.scan.as_mut()
                && (s.error().is_some()
                    || matches!(s.stage(), ScanStage::Unknown | ScanStage::WordsCaution))
            {
                s.resume();
                self.ui.set_scroll(ids::SCROLL, 0);
                return;
            }
            self.pop_screen();
            return;
        }
        // The first step of a combine is the Create wizard's count, and
        // what is behind it is the Load source row that opened it.
        if let Some(Wizard::Create(w)) = &self.wizard
            && w.is_combining()
            && w.step() == create::Step::Count
        {
            let mut load = LoadWizard::new();
            load.tap(ids::LOAD_SOURCE_XOR, self.network);
            self.wizard = Some(Wizard::Load(load));
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if matches!(&self.wizard, Some(Wizard::Backup(b))
            if b.step() == BackupStep::XorGather)
        {
            self.gather_back();
            return;
        }
        // Inside a SLIP-39 split: the gatherer's own steps first, then
        // the plan, then the flow that opened it.
        if self.gathering_shares() {
            self.share_gather_back();
            return;
        }
        // Inside a codex32 split: the same order.
        if self.gathering_codex32() {
            self.codex32_gather_back();
            return;
        }
        if self.on_codex32_plan() {
            if self.codex32.back() {
                self.ui.set_scroll(ids::SCROLL, 0);
                return;
            }
            self.codex32.zeroize();
            let stepped = match &mut self.wizard {
                Some(Wizard::Create(w)) => {
                    let more = w.back();
                    self.sync_create();
                    more
                }
                Some(Wizard::Backup(b)) => b.back(),
                _ => false,
            };
            if !stepped {
                self.cancel_wizard();
            }
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.on_share_plan() {
            if self.shares.back() {
                self.ui.set_scroll(ids::SCROLL, 0);
                return;
            }
            self.shares.zeroize();
            let stepped = match &mut self.wizard {
                Some(Wizard::Create(w)) => w.back(),
                Some(Wizard::Backup(b)) => b.back(),
                _ => false,
            };
            if !stepped {
                self.cancel_wizard();
            }
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if let Some(w) = &mut self.wizard {
            let stepped = match w {
                // Explore's word entry starts at the count step.
                Wizard::Load(w) if self.explore_entry && w.step() == Step::Count => false,
                Wizard::Load(w) => w.back(),
                Wizard::Build(w) => w.back(),
                Wizard::Create(w) => {
                    let more = w.back();
                    self.sync_create();
                    more
                }
                Wizard::Backup(b) => b.back(),
            };
            if !stepped {
                self.cancel_wizard();
            }
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        // §16.129: the passphrase entry a wallet being built opened goes
        // back to "Passphrase?", with Yes still checked.
        if matches!(self.screen, Screen::OpenPassphrase(_))
            && let Some(w) = self.build_parked.take()
        {
            self.pop_screen();
            self.wizard = Some(Wizard::Build(w));
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if let Screen::OpenChild(_) = self.screen
            && self.child.as_mut().is_some_and(ChildFlow::back)
        {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if let Screen::Bip85(_) = self.screen
            && self.bip85.as_mut().is_some_and(Bip85Flow::back)
        {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if let Screen::Vanity(_) = self.screen
            && self.vanity_back()
        {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.screen == Screen::Lightning && self.lightning_back() {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.screen == Screen::Sign && self.sign.back() {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.screen == Screen::SignMessage
            && let Some(flow) = self.message.as_mut()
            && flow.back()
        {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.screen == Screen::CheckedMessage
            && let Some(check) = self.check.as_mut()
            && check.back()
        {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.screen == Screen::Decode && self.decode.back() {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.screen == Screen::DicePassphrase && self.dice.back() {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.screen == Screen::WordList && self.word_list.step() == wordlist::Step::Search {
            self.word_list.go(wordlist::Step::Language);
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.screen == Screen::Explore && self.explore_back() {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if self.screen == Screen::Verify && self.verify.back() {
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        // One address has a screen of its own: Back closes it before it
        // leaves the list.
        if matches!(self.screen, Screen::Addresses(_)) && self.detail.open.is_some() {
            self.detail.open = None;
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        // §16.110 rule 2: the token and the description are steps of
        // the export, so the chevron walks back through them and the
        // format goes back to the one the screen can still show.
        if let Screen::KeyExport(key, account, step) = self.screen
            && step != KeyExportStep::Export
        {
            self.screen = match step {
                KeyExportStep::Description => Screen::KeyExport(key, account, KeyExportStep::Token),
                _ => {
                    self.export.format = ExportFormat::xpub_index();
                    Screen::KeyExport(key, account, KeyExportStep::Export)
                }
            };
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if matches!(self.screen, Screen::Export(_) | Screen::KeyExport(..)) && self.export.qr {
            self.export.qr = false;
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        // The Result over a code that could not be used, the unknown-code
        // menu and the words caution are all steps of the scanner: the
        // chevron puts the camera back on before it leaves.
        if self.screen == Screen::Scan
            && let Some(s) = self.scan.as_mut()
            && (s.error().is_some()
                || matches!(s.stage(), ScanStage::Unknown | ScanStage::WordsCaution))
        {
            s.resume();
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        self.pop_screen();
    }

    /// The screen behind this one. Home has nothing behind it: Back
    /// there is the way out of the app, pressed twice.
    fn pop_screen(&mut self) {
        match self.stack.pop() {
            Some(s) => {
                self.screen = s;
                self.entered();
            }
            None if self.screen == Screen::Home => self.back_at_root(),
            None => self.go_home(),
        }
    }

    /// Back where nothing is behind — Home, the lock screen, the stored
    /// key's pad. The first press says what a second one does, on the
    /// screen's caption line, for [`LEAVE_WINDOW_MS`]; a second press
    /// inside that window leaves (§16.73). The window is not a timer of
    /// the session: it neither restarts nor stops the lock and wipe
    /// timers, which every input already restarts.
    fn back_at_root(&mut self) {
        if self.leave_deadline.is_some_and(|d| self.now_ms < d) {
            self.leave();
        } else {
            self.leave_deadline = Some(self.now_ms + LEAVE_WINDOW_MS);
        }
    }

    /// Leaves the app: every key and cache leaves memory, the session
    /// and its PIN are reset, and the shell is asked to exit. A key kept
    /// on the device stays kept — leaving is not "Wipe and exit", which
    /// is the one action that forgets the device's copy — and the
    /// terminal screen says which of the two happened.
    fn leave(&mut self) {
        self.wipe_secrets();
        self.stack.clear();
        self.ended_by_leave = true;
        self.screen = Screen::Ended;
        self.entered();
        self.commands.push_back(Command::Exit);
    }

    /// States `text` on the screen's caption line for [`NOTICE_MS`].
    fn say(&mut self, text: &str) {
        self.notice = Some((String::from(text), self.now_ms + NOTICE_MS));
    }

    /// That line while it lasts. The Scanner draws it inside the
    /// viewfinder; every other screen draws it where its caption goes.
    pub(crate) fn notice(&self) -> Option<&str> {
        self.notice
            .as_ref()
            .filter(|(_, until)| self.now_ms < *until)
            .map(|(text, _)| text.as_str())
    }

    /// The public string the screen on top offers to copy (§4.10: a
    /// secret is never copied, so no screen here holds one). `None`
    /// where the screen has nothing to copy.
    pub(crate) fn copy_value(&self) -> Option<String> {
        if let Some(Overlay::Compare(cmp)) = &self.overlay {
            return Some(cmp.value.clone());
        }
        if self.overlay.is_some() {
            return None;
        }
        match &self.wizard {
            // The encrypted backup: ciphertext, which is public by
            // construction, in the text form a file carries.
            Some(Wizard::Backup(b)) if b.step() == BackupStep::Encrypted => {
                return Some(osk_psbt::base64::encode(b.backup()));
            }
            // The group record is public data, and copying it is how it
            // reaches a coordinator on a device with a clipboard.
            Some(Wizard::Build(w)) if w.step() == build::Step::Record => {
                return w.dealer().map(|d| d.record().to_text());
            }
            Some(_) => return None,
            None => {}
        }
        match self.screen {
            Screen::Addresses(owner) => {
                let index = self.detail.open? as usize;
                self.addresses_of(owner).get(index).cloned()
            }
            Screen::Export(_) | Screen::KeyExport(..) => self.export_value(self.export_owner()),
            Screen::Sign => self
                .sign
                .outcome()
                .map(|out| osk_psbt::base64::encode(&out.psbt)),
            Screen::SignMessage => {
                let flow = self.message.as_ref()?;
                let out = flow.outcome()?;
                Some(osk_psbt::message::signed_text(
                    &out.address,
                    &out.signature,
                    flow.text(),
                ))
            }
            _ => None,
        }
    }

    /// The Copy row a screen carries, where there is a public string on
    /// it. Dimmed with the reason on a shell known to have no clipboard.
    pub(crate) fn copy_row(&self) -> Option<osk_ui::screens::ActionRow> {
        self.copy_value().map(|_| osk_ui::screens::ActionRow {
            id: ids::COPY,
            icon: Icon::Copy,
            label: String::from(self.strings().action_copy),
            reason: self.clipboard_reason(),
        })
    }

    /// The bytes the one static code of the QR screen on top carries,
    /// where that code is public (`docs/PLANNING.md` §16.134 rule 4):
    /// an account key or a wallet export, a signed transaction, a
    /// message signature, a threshold group record, an address, a
    /// silent payment address, and an encrypted backup's or a sealed
    /// file's ciphertext. `None` on every other screen, and on every
    /// secret: a seed code and its grid are Secret screens, never these.
    pub(crate) fn public_code(&self) -> Option<Vec<u8>> {
        if self.overlay.is_some() {
            return None;
        }
        match &self.wizard {
            Some(Wizard::Backup(b)) if b.step() == BackupStep::EncryptedQr => {
                return Some(b.backup().to_vec());
            }
            Some(Wizard::Build(w)) if w.step() == build::Step::Record => {
                return w.dealer().map(|d| d.record().to_text().into_bytes());
            }
            Some(_) => return None,
            None => {}
        }
        match self.screen {
            Screen::SealedQr => Some(self.sealed.clone()),
            Screen::Addresses(owner) => {
                let index = self.detail.open? as usize;
                self.addresses_of(owner)
                    .get(index)
                    .map(|a| a.clone().into_bytes())
            }
            Screen::SilentAddress(wallet, label) => {
                self.silent_shown(wallet, label).map(String::into_bytes)
            }
            // The scan descriptor carries the scan private key, so its
            // format is never a code on this screen.
            Screen::Export(_) | Screen::KeyExport(_, _, KeyExportStep::Export)
                if self.export.qr && self.export_format() != ExportFormat::SilentScan =>
            {
                self.export_value(self.export_owner())
                    .map(String::into_bytes)
            }
            Screen::Sign if self.sign.stage() == Stage::Wizard(sign::Step::Qr) => {
                self.sign.static_payload()
            }
            Screen::SignMessage => {
                let flow = self.message.as_ref()?;
                if flow.stage() != message::Stage::Qr {
                    return None;
                }
                flow.outcome().map(|out| out.signature.clone().into_bytes())
            }
            _ => None,
        }
    }

    /// The "Save as PNG" row of the QR screen on top, where its code is
    /// public: live, or dimmed with the Animated toggle's reason where
    /// the content is too dense for one code.
    pub(crate) fn png_row(&self) -> Option<osk_ui::screens::ActionRow> {
        let bytes = self.public_code()?;
        Some(osk_ui::screens::ActionRow {
            id: ids::SAVE_PNG,
            icon: Icon::File,
            label: String::from(self.strings().action_save_png),
            reason: (bytes.len() > sign::STATIC_QR_MAX_BYTES)
                .then(|| String::from(self.strings().sign_qr_dense)),
        })
    }

    /// The same row on the Address screen, which a 268 dp panel does
    /// not carry: there the whole address takes the room a row would,
    /// and §4.5 shows an address whole before anything else.
    pub(crate) fn address_png_row(
        &self,
        c: &osk_ui::screens::Chrome<'_>,
    ) -> Option<osk_ui::screens::ActionRow> {
        if c.class() == SizeClass::Small {
            return None;
        }
        self.png_row()
    }

    /// Hands the shell the code on screen as a PNG: the one static code,
    /// black on white with its quiet zone, [`PNG_MODULE_PX`] pixels to a
    /// module.
    fn save_png(&mut self) {
        if self.png_save == Save::Waiting {
            return;
        }
        let Some(bytes) = self.public_code() else {
            return;
        };
        if bytes.len() > sign::STATIC_QR_MAX_BYTES {
            return;
        }
        let Ok(matrix) = osk_codec::qr::encode(
            osk_codec::qr::Payload::Bytes(&bytes),
            osk_codec::qr::Ecc::Low,
        ) else {
            return;
        };
        self.png_save = Save::Waiting;
        self.commands.push_back(Command::WriteFile {
            kind: FileKind::Png,
            name_hint: String::from(self.strings().png_file_name),
            bytes: osk_codec::png::qr_png(&matrix, PNG_MODULE_PX),
        });
    }

    /// The same row as a Record, for the result screens that carry
    /// their ways on as table rows (§4.11).
    pub(crate) fn copy_record(&self) -> Option<osk_ui::components::Record> {
        self.copy_row().map(|row| match row.reason {
            Some(reason) => osk_ui::components::Record::dimmed(row.icon, row.label, Some(reason)),
            None => osk_ui::components::Record::action(row.id, row.icon, row.label),
        })
    }

    /// Whether this shell is known to have no clipboard, which dims the
    /// Paste and Copy rows with the reason.
    pub(crate) fn clipboard_reason(&self) -> Option<String> {
        (!self.has_clipboard).then(|| String::from(self.strings().reason_no_clipboard))
    }

    /// The caption the first Back at the root shows, while its window is
    /// open.
    pub(crate) fn leave_notice(&self) -> Option<String> {
        self.leave_deadline
            .is_some_and(|d| self.now_ms < d)
            .then(|| String::from(self.strings().leave_again))
    }

    fn start_load(&mut self) {
        self.wizard = Some(Wizard::Load(LoadWizard::new()));
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    fn start_build(&mut self) {
        self.wizard = Some(Wizard::Build(WalletWizard::new()));
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// A key page's "Add a wallet": the one wizard, with that key in
    /// hand, opening on "Passphrase?" (§16.129 rules 1 and 2).
    fn start_build_from(&mut self, key: usize) {
        let mut w = WalletWizard::from_key(key);
        // A key whose words this device does not hold — SLIP-39 shares,
        // Codex32 — has no passphrase to add, so the question is not
        // asked and the wizard opens on the kind.
        if !self.keys.get(key).is_some_and(LoadedKey::has_mnemonic) {
            w.go(build::Step::Kind);
        }
        self.wizard = Some(Wizard::Build(w));
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// Continue on the kind step. With a key in hand that key is checked
    /// first; a kind built on one key then goes on past the Keys step,
    /// and every other kind opens it with the key checked (§16.129
    /// rule 3). A key in hand this kind cannot use leaves the Keys step
    /// to say why.
    fn build_kind_continue(&mut self) {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        let mut skip = false;
        if let Some(hand) = w.in_hand() {
            let usable = self.build_key_reason(w, hand).is_none();
            if usable
                && !w.holds_loaded(hand)
                && w.can_add()
                && let Some(key) = self.build_key_of(hand, w)
            {
                self.with_build(|w| {
                    let _ = w.add(key, Some(hand), None);
                });
            }
            let Some(Wizard::Build(w)) = &self.wizard else {
                return;
            };
            skip = w.kind().one_key() && w.holds_loaded(hand);
        }
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        let next = if skip { w.after_keys() } else { w.after_kind() };
        self.with_build(|w| {
            w.set_keys_skipped(skip);
            w.go(next);
        });
        if skip {
            self.build_rebuild_keys();
        }
    }

    /// Continue on "Passphrase?". No goes on to the kind with the key in
    /// hand; Yes sets the wizard aside and opens that key's passphrase
    /// entry, whose ✓ adds the key the wizard goes on over.
    fn build_passphrase_continue(&mut self) {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        let Some(key) = w.in_hand() else {
            return;
        };
        if !w.passphrase() {
            self.with_build(|w| w.go(build::Step::Kind));
            return;
        }
        if !self.keys.get(key).is_some_and(LoadedKey::has_mnemonic) {
            return;
        }
        let Some(Wizard::Build(w)) = self.wizard.take() else {
            return;
        };
        self.picked_key = Some(key);
        self.push(Screen::OpenPassphrase(key));
        self.open_finish = Some(Finish::new());
        self.build_parked = Some(w);
    }

    /// The passphrase key opened for a wallet being built: it joins Keys
    /// as a peer, as "Open with passphrase" adds one, and the wizard
    /// goes on to the kind with it in hand. The entry is replaced by the
    /// new key's page, which is where the wizard's way out leads.
    fn build_passphrase_opened(&mut self, key: LoadedKey, mut w: WalletWizard) {
        let index = match self
            .keys
            .iter()
            .position(|k| k.fingerprint == key.fingerprint)
        {
            Some(i) => i,
            None => {
                let mut key = key;
                key.seal(self.session.key_mut());
                self.keys.push(key);
                self.keys.len() - 1
            }
        };
        self.stack.pop();
        self.screen = Screen::KeyDetail(index);
        self.entered();
        w.set_in_hand(index);
        w.go(build::Step::Kind);
        self.wizard = Some(Wizard::Build(w));
    }

    /// Whether the scanner is open over the wallet builder, which is the
    /// one wizard a scan comes back to.
    fn building_scan(&self) -> bool {
        self.screen == Screen::Scan
            && match &self.wizard {
                Some(Wizard::Build(_)) => true,
                // The Create wizard's Seed XOR parts arrive through the
                // scanner too, one per part.
                Some(Wizard::Create(w)) => w.step() == create::Step::XorPart,
                _ => false,
            }
    }

    /// Does `f` to the wallet wizard, when that is the wizard.
    fn with_build(&mut self, f: impl FnOnce(&mut WalletWizard)) {
        if let Some(Wizard::Build(w)) = &mut self.wizard {
            f(w);
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// One loaded key's contribution to a wallet of this shape: its
    /// BIP-48 multisig account, its BIP-86 account for a MuSig2 wallet,
    /// or its account at the chosen script type for a single-sig one,
    /// with the origin it derives under.
    fn build_key_of(&self, key: usize, w: &WalletWizard) -> Option<build::PolicyKey> {
        let master = self.keys.get(key)?.master.as_ref()?;
        let kind = w.account_kind();
        let text = match w.kind() {
            build::WalletKind::Single => {
                let account = master.account_xpub(w.script(), 0).ok()?;
                build::account_key_h(account.master_fingerprint(), account.path(), account.xpub())
            }
            _ => match kind.multisig() {
                Some(script) => {
                    let account = master.multisig_account_xpub(script, 0).ok()?;
                    build::account_key(account.master_fingerprint(), account.path(), account.xpub())
                }
                None => {
                    let account = master.account_xpub(ScriptType::Taproot, 0).ok()?;
                    build::account_key(account.master_fingerprint(), account.path(), account.xpub())
                }
            },
        };
        build::read_key(&text, kind, self.network).ok()
    }

    /// Why the loaded key at `i` cannot be a key of a wallet of `kind`,
    /// in the two or three words §4.11 gives a reason. `None` where it
    /// can.
    pub(crate) fn build_key_reason(
        &self,
        w: &WalletWizard,
        i: usize,
    ) -> Option<alloc::string::String> {
        let s = self.strings();
        let key = self.keys.get(i)?;
        // A recovery wallet's two paths may not share a key: one that
        // spends both ways makes the review of who can spend a lie.
        if w.held_elsewhere(i) {
            return Some(String::from(s.build_reason_other_path));
        }
        // §16.128 rule 1: a kind that is full of several keys takes no
        // more, so the rows it cannot take say so. A kind built on one
        // key moves its check instead, and dims nothing.
        if !w.kind().one_key() && !w.can_add() && !w.holds_loaded(i) {
            return Some(String::from(s.build_reason_group_full));
        }
        // §16.113: a silent payments wallet is one key's scan and spend
        // halves, and only a key this device holds has both.
        if w.kind() == build::WalletKind::Silent {
            return key
                .master
                .is_none()
                .then(|| String::from(s.build_reason_not_usable));
        }
        if w.kind() != build::WalletKind::Frost {
            return None;
        }
        // A FROST group's keys are 24 words whose 32 bytes are a scalar,
        // and this device must hold those words to use one.
        if key.share.is_some() {
            return None;
        }
        let count = key.mnemonic(self.session.key(), Mnemonic::word_count);
        Some(match count {
            Some(n) if n != 24 => strings::fill1(s.build_reason_words, &alloc::format!("{n}")),
            _ => String::from(s.build_reason_not_usable),
        })
    }

    /// Why a key read at the scanner could not join the wallet, in the
    /// two or three words §4.11 gives a reason.
    fn build_reason(&self, refusal: build::Refusal) -> &'static str {
        let s = self.strings();
        match refusal {
            build::Refusal::NotAKey => s.scan_reason_type,
            build::Refusal::Wallet => s.build_reason_wallet,
            build::Refusal::NoOrigin => s.build_reason_origin,
            build::Refusal::Mainnet => s.build_reason_mainnet,
            build::Refusal::Testnet => s.build_reason_testnet,
            build::Refusal::Duplicate => s.build_reason_added,
        }
    }

    /// A cosigner's key read at the scanner: it joins the wallet being
    /// built and the scanner closes, or it is refused where it was read.
    fn cosigner_scanned(&mut self, text: &str) {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        let kind = w.account_kind();
        let read = build::read_key(text, kind, self.network);
        let source = String::from(text);
        let outcome = match read {
            Ok(key) => match &mut self.wizard {
                Some(Wizard::Build(w)) => w.add(key, None, Some(source)),
                _ => return,
            },
            Err(refusal) => Err(refusal),
        };
        let typing =
            matches!(&self.wizard, Some(Wizard::Build(w)) if w.step() == build::Step::Type);
        match outcome {
            // Leaving the scanner turns the camera off and puts the
            // wizard back on its Keys step, where the new key is. A key
            // that was typed is already on the wizard's own screen.
            Ok(()) if typing => {
                let back = match &self.wizard {
                    Some(Wizard::Build(w)) => w.typing_from(),
                    _ => build::Step::Keys,
                };
                self.with_build(|w| w.go(back));
            }
            Ok(()) => self.pop_screen(),
            Err(refusal) => {
                let reason = String::from(self.build_reason(refusal));
                if typing {
                    self.with_build(|w| w.set_type_refusal(refusal));
                } else {
                    self.scan_failed(reason);
                }
            }
        }
    }

    /// Add a wallet's controls, step by step.
    fn tap_build(&mut self, id: Id) {
        use build::Step;
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        let (step, keys, ready, can_add) = (w.step(), w.keys().len(), w.ready(), w.can_add());
        match step {
            Step::Kind => {
                if let Some(i) =
                    ids::index_in(id, ids::BUILD_KIND_BASE, build::WalletKind::ALL.len())
                {
                    self.with_build(|w| w.set_kind(i));
                } else if id == ids::BUILD_KIND_CONTINUE {
                    self.build_kind_continue();
                }
            }
            Step::Count => {
                if let Some(i) = ids::index_in(id, ids::THRESHOLD_COUNT_BASE, COUNTS.len()) {
                    self.with_build(|w| w.set_n(COUNTS[i]));
                } else if id == ids::THRESHOLD_COUNT_CONTINUE {
                    self.with_build(|w| w.go(Step::Quorum));
                }
            }
            Step::Quorum => {
                if let Some(i) =
                    ids::index_in(id, ids::THRESHOLD_QUORUM_BASE, threshold::MAX_SHARES)
                {
                    self.with_build(|w| w.set_t(i as u8 + 2));
                } else if id == ids::THRESHOLD_QUORUM_CONTINUE {
                    self.with_build(|w| w.go(Step::Keys));
                }
            }
            Step::Keys => {
                if let Some(i) = ids::index_in(id, ids::BUILD_WHICH_BASE, self.keys.len()) {
                    self.build_toggle_key(i);
                } else if let Some(i) = ids::index_in(id, ids::BUILD_KEY_ROW_BASE, keys) {
                    self.with_build(|w| w.remove(i));
                } else if id == ids::BUILD_WHICH_SCAN && can_add {
                    self.start_scan(Expect::Cosigner);
                } else if id == ids::BUILD_CONTINUE && ready {
                    let next = w.after_keys();
                    self.with_build(|w| w.go(next));
                    self.build_rebuild_keys();
                }
            }
            Step::Later => {
                if let Some(i) = ids::index_in(id, ids::BUILD_WHICH_BASE, self.keys.len()) {
                    self.build_toggle_key(i);
                } else if let Some(i) = ids::index_in(id, ids::BUILD_KEY_ROW_BASE, keys) {
                    self.with_build(|w| w.remove(i));
                } else if id == ids::BUILD_WHICH_SCAN && can_add {
                    self.start_scan(Expect::Cosigner);
                } else if id == ids::BUILD_LATER_CONTINUE && ready {
                    let next = w.after_later();
                    self.with_build(|w| w.go(next));
                }
            }
            Step::LaterThreshold => {
                let n = w.later_keys().len();
                if let Some(i) = ids::index_in(id, ids::BUILD_LATER_THRESHOLD_BASE, n) {
                    self.with_build(|w| w.set_later_threshold(i + 1));
                } else if id == ids::BUILD_LATER_THRESHOLD_CONTINUE {
                    self.with_build(|w| w.go(Step::Delay));
                }
            }
            Step::Delay => {
                if let Some(i) = ids::index_in(id, ids::BUILD_DELAY_BASE, build::DELAYS.len()) {
                    self.with_build(|w| w.set_days(build::DELAYS[i]));
                } else if id == ids::BUILD_DELAY_TYPE {
                    self.with_build(WalletWizard::set_typed_wait);
                } else if id == ids::BUILD_DELAY_CONTINUE {
                    // The typed row's number is asked for on its own pad;
                    // the four offered waits need nothing more.
                    let next = if w.typed_wait() {
                        Step::Days
                    } else {
                        w.after_delay()
                    };
                    self.with_build(|w| w.go(next));
                }
            }
            // The pad's one control is its keyboard.
            Step::Days => {}
            Step::Another => {
                if id == ids::BUILD_ANOTHER_NO {
                    self.with_build(|w| w.set_another(false));
                } else if id == ids::BUILD_ANOTHER_YES {
                    self.with_build(|w| w.set_another(true));
                } else if id == ids::BUILD_ANOTHER_CONTINUE {
                    let next = w.after_another();
                    self.with_build(|w| {
                        if next == Step::Later {
                            w.add_path();
                        }
                        w.go(next);
                    });
                }
            }
            // The entry's one control is its keyboard.
            Step::Type => {}
            Step::Script => {
                if let Some(i) = ids::index_in(id, ids::BUILD_SCRIPT_BASE, ScriptType::ALL.len()) {
                    self.with_build(|w| w.set_script(i));
                    self.build_rebuild_keys();
                } else if id == ids::BUILD_SCRIPT_CONTINUE {
                    let next = w.after_script();
                    self.with_build(|w| w.go(next));
                }
            }
            Step::Threshold => {
                let keys = w.primary_keys().len();
                if let Some(i) = ids::index_in(id, ids::BUILD_THRESHOLD_BASE, keys) {
                    self.with_build(|w| w.set_threshold(i + 1));
                } else if id == ids::BUILD_THRESHOLD_CONTINUE {
                    let next = w.after_threshold();
                    self.with_build(|w| w.go(next));
                }
            }
            Step::Review => {
                if id == ids::BUILD_ADD_WALLET {
                    self.build_add_wallet();
                } else if id == ids::INSPECT_TEXT {
                    let text = match &self.wizard {
                        Some(Wizard::Build(w)) => w.policy().map(|p| p.to_descriptor_checksummed()),
                        _ => None,
                    };
                    if let Some(text) = text {
                        self.open_compare_descriptor(self.strings().inspect_descriptor, text);
                    }
                }
            }
            Step::Words | Step::QuizStart | Step::Quiz | Step::QuizSkip | Step::Record => {
                self.tap_deal(id);
            }
            Step::Passphrase => {
                if id == ids::BUILD_PASSPHRASE_NO {
                    self.with_build(|w| w.set_passphrase(false));
                } else if id == ids::BUILD_PASSPHRASE_YES {
                    self.with_build(|w| w.set_passphrase(true));
                } else if id == ids::BUILD_PASSPHRASE_CONTINUE {
                    self.build_passphrase_continue();
                }
            }
            Step::Discard => {
                if id == ids::BUILD_DISCARD {
                    self.cancel_wizard();
                } else if id == ids::BUILD_KEEP {
                    self.with_build(WalletWizard::keep);
                }
            }
        }
    }

    /// A row of the Keys step: the loaded key at `which` joins the
    /// wallet, and a second tap takes it back out.
    fn build_toggle_key(&mut self, which: usize) {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        if w.holds_loaded(which) {
            self.with_build(|w| w.remove_loaded(which));
            return;
        }
        if self.build_key_reason(w, which).is_some() {
            return;
        }
        // §16.128 rule 1: on a kind built on one key the check moves,
        // so the key already on the wallet comes off to make room.
        let moves = w.kind().one_key();
        if !moves && !w.can_add() {
            return;
        }
        let Some(key) = self.build_key_of(which, w) else {
            return;
        };
        self.with_build(|w| {
            if moves {
                w.clear_keys();
            }
            let _ = w.add(key, Some(which), None);
        });
    }

    /// The script type decides which account every key contributes, and
    /// it is chosen after the keys, so the gathered keys are read again
    /// whenever it changes.
    fn build_rebuild_keys(&mut self) {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        let kind = w.account_kind();
        let network = self.network;
        let rebuilt: Vec<build::BuiltKey> = w
            .primary_keys()
            .iter()
            .map(|k| {
                let key = match k.loaded() {
                    Some(i) => self.build_key_of(i, w),
                    None => k
                        .source()
                        .and_then(|text| build::read_key(text, kind, network).ok()),
                };
                match key {
                    Some(key) => WalletWizard::rebuilt(k, key),
                    None => k.clone(),
                }
            })
            .collect();
        let later: Vec<Vec<build::BuiltKey>> = (0..w.later_paths())
            .map(|path| {
                w.later_path_keys(path)
                    .iter()
                    .map(|k| {
                        let key = match k.loaded() {
                            Some(i) => self.build_key_of(i, w),
                            None => k
                                .source()
                                .and_then(|text| build::read_key(text, kind, network).ok()),
                        };
                        match key {
                            Some(key) => WalletWizard::rebuilt(k, key),
                            None => k.clone(),
                        }
                    })
                    .collect()
            })
            .collect();
        self.with_build(|w| {
            w.set_keys(rebuilt);
            for (path, keys) in later.into_iter().enumerate() {
                w.set_later_keys(path, keys);
            }
        });
    }

    /// "Add this wallet": the policy joins the wallets in use, or, where
    /// an equal one is already there, that one opens instead of a
    /// second. A FROST review deals its group instead, and the words of
    /// every computed key follow.
    fn build_add_wallet(&mut self) {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        if w.kind() == build::WalletKind::Frost {
            self.deal_group();
            return;
        }
        // §16.113: a silent payments wallet is the two public keys
        // BIP-352 derives from the chosen key, with no descriptor and
        // no address at an index.
        if w.kind() == build::WalletKind::Silent {
            let Some(record) = self.silent_built() else {
                return;
            };
            self.add_built_wallet(osk_bip::policy::WalletPolicy::of_silent(record));
            return;
        }
        // A wallet over one key exports that key's account, which the
        // Export screen reads out of the account cache, so the cache is
        // warmed here where the script type is settled.
        if w.kind() == build::WalletKind::Single
            && let Some(key) = w.keys().first().and_then(build::BuiltKey::loaded)
        {
            let script = w.script();
            self.account(key, script);
        }
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        let Some(policy) = w.policy() else {
            return;
        };
        self.add_built_wallet(policy);
    }

    /// The wallet the review described, put in use: a second copy of
    /// one already there opens that one instead.
    fn add_built_wallet(&mut self, policy: osk_bip::policy::WalletPolicy) {
        let (wallet, ask) = match self.wallets.iter().position(|p| *p == policy) {
            Some(i) => (i, false),
            None => {
                self.wallets.push(policy);
                let i = self.wallets.len() - 1;
                // The answer is "No" from the moment the wallet exists,
                // so a wallet over a passphrase key is never written
                // before the person has said to write it.
                let ask = self.asks_to_keep_wallet(i);
                if ask {
                    self.set_wallet_kept(i, false);
                } else {
                    self.sync_kept();
                }
                (i, ask)
            }
        };
        self.cancel_wizard();
        self.open_wallet(wallet);
        // §5 Choice, "Keep this wallet on the device?", over the page of
        // the wallet just added (§16.104 rule 7).
        if ask {
            self.overlay = Some(Overlay::Choice(Picker::KeepWallet, 0));
        }
    }

    fn start_create(&mut self) {
        self.open_create(CreateWizard::new());
    }

    /// Add a key › "Create SLIP-39 shares": the same source Choice and
    /// the same entropy screens, over a master secret that is only ever
    /// written as shares (`docs/PLANNING.md` §16.107 rule 4).
    fn start_create_shares(&mut self) {
        self.shares.zeroize();
        self.open_create(CreateWizard::shares());
    }

    fn open_create(&mut self, w: CreateWizard) {
        let mut w = w;
        // What this build can take, which dims two of the source rows
        // (§4.11): a shell that said it has no camera, and Tier D,
        // where the generator belongs to a browser.
        w.set_device(self.has_camera, self.tier == AssuranceTier::D);
        self.wizard = Some(Wizard::Create(w));
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    // ----- the SLIP-39 split (`crate::shares`, §16.107 rules 4 and 5) -----

    /// Whether a random value of the split is being gathered now.
    fn gathering_shares(&self) -> bool {
        match &self.wizard {
            Some(Wizard::Create(w)) => w.is_slip39() && w.share_gather().is_some(),
            Some(Wizard::Backup(b)) => {
                b.step() == BackupStep::Shares
                    && b.gather().is_some_and(|g| g.share_gather().is_some())
            }
            _ => false,
        }
    }

    /// Backup › "SLIP-39 shares": the key's master secret, which for a
    /// SLIP-39 key is its seed, split again under a plan and a
    /// passphrase of this backup's own (§16.107 rule 5).
    fn start_share_backup(&mut self, key: usize) {
        self.shares.zeroize();
        let source = osk_entropy::SOURCE_ROWS[0];
        let started = self
            .keys
            .get(key)
            .and_then(|k| k.seed(self.session.key(), |seed| seed.to_vec()));
        let Some(mut seed) = started else {
            return;
        };
        let ok = self.shares.start_backup(&seed, source);
        seed.zeroize();
        if !ok {
            self.shares.zeroize();
            return;
        }
        self.wizard = Some(Wizard::Backup(BackupFlow::new(key, BackupStep::Shares)));
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// A tap inside a Backup's share gatherer, which draws the Create
    /// wizard's own screens.
    fn tap_share_gather(&mut self, id: Id, network: Network) {
        let now = self.now_ms;
        let class = self.class();
        let pane = self.words_pane_dp();
        let Some(Wizard::Backup(b)) = &mut self.wizard else {
            return;
        };
        let Some(g) = b.gather_mut() else {
            return;
        };
        if id == ids::CREATE_SHUTTER && g.step() == create::Step::Camera {
            self.create_shutter();
            self.sync_create();
            self.render();
            return;
        }
        if g.step() == create::Step::MixResult
            && let Some(i) = ids::index_in(id, ids::CREATE_MIX_BASE, osk_entropy::MIX_SOURCES.len())
            && let Some(c) = g.mix_commitments().get(i).copied()
        {
            let title = self.strings().create_mixed_title;
            self.open_compare(title, text::hex(&c));
            return;
        }
        let per_page = views::words::per_page(class, pane, g.language());
        g.tap(id, network, now, per_page);
        let ready = g.step() == create::Step::Words;
        self.sync_create();
        if ready {
            self.share_gathered();
            self.render();
            return;
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// A tap inside a Backup's codex32 gatherer, which draws the Create
    /// wizard's own screens.
    fn tap_codex32_gather(&mut self, id: Id, network: Network) {
        let now = self.now_ms;
        let class = self.class();
        let pane = self.words_pane_dp();
        let Some(Wizard::Backup(b)) = &mut self.wizard else {
            return;
        };
        let Some(g) = b.gather_mut() else {
            return;
        };
        if id == ids::CREATE_SHUTTER && g.step() == create::Step::Camera {
            self.create_shutter();
            self.sync_create();
            self.render();
            return;
        }
        if g.step() == create::Step::MixResult
            && let Some(i) = ids::index_in(id, ids::CREATE_MIX_BASE, osk_entropy::MIX_SOURCES.len())
            && let Some(c) = g.mix_commitments().get(i).copied()
        {
            let title = self.strings().create_mixed_title;
            self.open_compare(title, text::hex(&c));
            return;
        }
        let per_page = views::words::per_page(class, pane, g.language());
        g.tap(id, network, now, per_page);
        let ready = g.step() == create::Step::Words;
        self.sync_create();
        if ready {
            self.codex32_gathered();
            self.render();
            return;
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// Whether the plan owns the screen now: the flow is on its shares
    /// step and the plan is not inside the gatherer, whose screens are
    /// the Create wizard's.
    fn on_share_plan(&self) -> bool {
        let on_step = match &self.wizard {
            Some(Wizard::Create(w)) => w.step() == create::Step::Shares,
            Some(Wizard::Backup(b)) => b.step() == BackupStep::Shares,
            _ => false,
        };
        on_step && self.shares.step() != ShareStep::Gather
    }

    /// The Create wizard has reached the plan: load it with the master
    /// secret and the passphrase the shares will be written under.
    fn sync_shares(&mut self) {
        let Some(Wizard::Create(w)) = &self.wizard else {
            return;
        };
        if w.step() != create::Step::Shares || self.shares.is_running() {
            return;
        }
        let source = w.source();
        // Both are inline in the wizard, so they are copied out into
        // the plan and the copies erased when it is left.
        let mut secret = [0u8; osk_bip::slip39::MAX_STRENGTH_BYTES];
        let mut pass = [0u8; shares::MAX_PASSPHRASE_BYTES];
        let mut lens = None;
        w.secret(|bytes| {
            secret[..bytes.len()].copy_from_slice(bytes);
            lens = Some((bytes.len(), 0));
        });
        if let Some((n, _)) = lens {
            let m = w.finish().passphrase(|bytes| {
                pass[..bytes.len()].copy_from_slice(bytes);
                bytes.len()
            });
            lens = Some((n, m));
        }
        let started = match lens {
            Some((n, m)) => self.shares.start_create(&secret[..n], &pass[..m], source),
            None => false,
        };
        secret.zeroize();
        pass.zeroize();
        if !started {
            self.cancel_wizard();
            return;
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// A tap on one of the plan's own screens.
    fn tap_shares(&mut self, id: Id) {
        let now = self.now_ms;
        let per_page =
            views::words::per_page_list(self.class(), self.words_pane_dp(), EntryList::Slip39);
        let next = self.shares.tap(id, now, per_page);
        self.after_shares(next);
    }

    /// What a tap on the plan asked for.
    fn after_shares(&mut self, next: ShareNext) {
        match next {
            ShareNext::Stay => self.ui.set_scroll(ids::SCROLL, 0),
            ShareNext::Gather => self.start_share_gather(),
            ShareNext::Split => self.make_shares(),
            ShareNext::Done => self.shares_finished(),
        }
    }

    /// Opens the chosen source's own entropy screens for the random
    /// value now due, titled by where it belongs in the plan (§16.92).
    fn start_share_gather(&mut self) {
        let (group, at, of) = self.shares.gather_label();
        let source = self.shares.source();
        let count = if self.shares.value_len() >= 32 {
            24
        } else {
            12
        };
        let (has_camera, tier_d) = (self.has_camera, self.tier == AssuranceTier::D);
        match &mut self.wizard {
            // Create gathers on the wizard's own screens: the source is
            // the one the key was just made from.
            Some(Wizard::Create(w)) => {
                w.start_share_gather(group, at, of);
            }
            Some(Wizard::Backup(b)) => {
                let mut g =
                    CreateWizard::gathering(source, count, osk_bip::bip39::Language::English, 0, 0);
                g.set_device(has_camera, tier_d);
                g.start_share_gather(group, at, of);
                b.set_gather(g);
            }
            _ => return,
        }
        self.sync_create();
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The gatherer has reached its words, which are one random value.
    fn share_gathered(&mut self) {
        let taken = match &mut self.wizard {
            Some(Wizard::Create(w)) => {
                let bytes = w.mnemonic().map(entropy_bytes);
                w.end_share_gather();
                w.go(create::Step::Shares);
                bytes
            }
            Some(Wizard::Backup(b)) => {
                let bytes = b
                    .gather()
                    .and_then(CreateWizard::mnemonic)
                    .map(entropy_bytes);
                b.drop_gather();
                bytes
            }
            _ => None,
        };
        let Some((mut buf, n)) = taken else {
            self.cancel_wizard();
            return;
        };
        let next = self.shares.take_random(&buf[..n]);
        buf.zeroize();
        self.sync_create();
        self.after_shares(next);
    }

    /// The chevron inside the gatherer: its own steps first, then the
    /// plan, with every value gathered so far forgotten.
    fn share_gather_back(&mut self) {
        let leaving = match &mut self.wizard {
            Some(Wizard::Create(w)) => {
                let more = w.back();
                if !more {
                    w.end_share_gather();
                    w.go(create::Step::Shares);
                }
                !more
            }
            Some(Wizard::Backup(b)) => match b.gather_mut() {
                Some(g) => {
                    let more = g.back();
                    if !more {
                        b.drop_gather();
                    }
                    !more
                }
                None => true,
            },
            _ => true,
        };
        if leaving && !self.shares.back() {
            self.cancel_wizard();
            return;
        }
        self.sync_create();
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// Every random value is in: the split itself. The identifier is
    /// public and stated in every share, so it comes from the shell's
    /// own randomness through the session key, mixed with the clock and
    /// with how many backups this session has made, so that two backups
    /// of one key are two backups.
    fn make_shares(&mut self) {
        let mut seed = [0u8; 16];
        seed[..4].copy_from_slice(&self.session.scramble_seed().unwrap_or(0).to_le_bytes());
        seed[4..12].copy_from_slice(&self.now_ms.to_le_bytes());
        seed[12..].copy_from_slice(&self.share_backups.to_le_bytes());
        let digest = osk_crypto::sha256(&seed);
        let identifier = u16::from_be_bytes([digest[0], digest[1]]) & 0x7fff;
        self.share_backups = self.share_backups.wrapping_add(1);
        if !self.shares.split(identifier) {
            self.cancel_wizard();
            return;
        }
        match &mut self.wizard {
            Some(Wizard::Create(w)) => w.go(create::Step::Shares),
            Some(Wizard::Backup(_)) => {}
            _ => {}
        }
        self.sync_create();
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The last share has been shown, and the Result behind it where the
    /// flow has one: Create goes on to Confirm, Backup closes.
    fn shares_finished(&mut self) {
        let verified = self.shares.verified();
        match &mut self.wizard {
            Some(Wizard::Create(w)) => {
                w.shares_made(verified);
                self.ui.set_scroll(ids::SCROLL, 0);
            }
            Some(Wizard::Backup(b)) => {
                let key = b.key();
                if let Some(k) = self.keys.get_mut(key) {
                    k.backup_verified = verified;
                }
                self.cancel_wizard();
                self.sync_kept();
            }
            _ => {}
        }
    }

    // ----- the codex32 strings (`crate::codex32`, §16.109 rules 4 and 5) -----

    /// Add a key › "Create Codex32 shares": the same source Choice and
    /// the same entropy screens, over a master seed that is only ever
    /// written as codex32 strings (`docs/PLANNING.md` §16.109 rule 4).
    fn start_create_codex32(&mut self) {
        self.codex32.zeroize();
        self.open_create(CreateWizard::codex32());
    }

    /// Backup › "Codex32": the key's seed, whatever the key was read
    /// from — 64 bytes from words with the passphrase already in them,
    /// 16 or 32 from SLIP-39 shares, any of BIP 93's six from a codex32
    /// string (§16.109 rule 5).
    fn start_codex32_backup(&mut self, key: usize) {
        self.codex32.zeroize();
        let source = osk_entropy::SOURCE_ROWS[0];
        let Some(k) = self.keys.get(key) else {
            return;
        };
        let words = k.has_mnemonic();
        let Some(mut seed) = k.seed(self.session.key(), |seed| seed.to_vec()) else {
            return;
        };
        let identifier = self.codex32_identifier(&seed);
        let ok = self.codex32.start_backup(&seed, source, identifier, words);
        seed.zeroize();
        if !ok {
            self.codex32.zeroize();
            return;
        }
        self.wizard = Some(Wizard::Backup(BackupFlow::new(key, BackupStep::Codex32)));
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The identifier every string of a set carries: BIP 93 leaves it
    /// undefined and asks only that it be distinct per secret, so it is
    /// computed from the master fingerprint of the seed and never asked
    /// for (§16.109 rule 4).
    fn codex32_identifier(&self, seed: &[u8]) -> [u8; 4] {
        let network = self.network;
        let print = osk_crypto::SeedBytes::new(seed).map(|bytes| {
            osk_bip::keys::MasterKey::from_seed_bytes(&osk_crypto::Secret::new(bytes), network)
                .fingerprint()
        });
        osk_bip::codex32::identifier_for(print.map_or([0; 4], |f| *f.as_bytes()))
    }

    /// Whether a run of the codex32 split's randomness is being
    /// gathered now.
    fn gathering_codex32(&self) -> bool {
        match &self.wizard {
            Some(Wizard::Create(w)) => w.is_codex32() && w.share_gather().is_some(),
            Some(Wizard::Backup(b)) => {
                b.step() == BackupStep::Codex32
                    && b.gather().is_some_and(|g| g.share_gather().is_some())
            }
            _ => false,
        }
    }

    /// Whether the codex32 plan owns the screen now.
    fn on_codex32_plan(&self) -> bool {
        let on_step = match &self.wizard {
            Some(Wizard::Create(w)) => w.step() == create::Step::Codex32,
            Some(Wizard::Backup(b)) => b.step() == BackupStep::Codex32,
            _ => false,
        };
        on_step && self.codex32.step() != Codex32Step::Gather
    }

    /// The Create wizard has reached the plan: load it with the seed the
    /// entropy came to.
    fn sync_codex32(&mut self) {
        let Some(Wizard::Create(w)) = &self.wizard else {
            return;
        };
        if w.step() != create::Step::Codex32 || self.codex32.is_running() {
            return;
        }
        let source = w.source();
        let mut seed = [0u8; osk_bip::codex32::MAX_SEED_BYTES];
        let mut len = None;
        w.secret(|bytes| {
            seed[..bytes.len()].copy_from_slice(bytes);
            len = Some(bytes.len());
        });
        let started = match len {
            Some(n) => {
                let identifier = self.codex32_identifier(&seed[..n]);
                self.codex32.start_create(&seed[..n], source, identifier)
            }
            None => false,
        };
        seed.zeroize();
        if !started {
            self.cancel_wizard();
            return;
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// A tap on one of the plan's own screens.
    fn tap_codex32(&mut self, id: Id) {
        let next = self.codex32.tap(id);
        self.after_codex32(next);
    }

    /// What a tap or a key on the plan asked for.
    fn after_codex32(&mut self, next: Codex32Next) {
        match next {
            Codex32Next::Stay => self.ui.set_scroll(ids::SCROLL, 0),
            Codex32Next::Gather => self.start_codex32_gather(),
            Codex32Next::Split => self.make_codex32(),
            Codex32Next::Done => self.codex32_finished(),
        }
    }

    /// Opens the chosen source's own entropy screens for the run now
    /// due (§16.92). A random share of a 64-byte seed takes two runs,
    /// because no source makes 512 bits at once.
    fn start_codex32_gather(&mut self) {
        let (at, of) = self.codex32.gather_label();
        let source = self.codex32.source();
        let count = self.codex32.gather_strength().words() as u8;
        let (has_camera, tier_d) = (self.has_camera, self.tier == AssuranceTier::D);
        match &mut self.wizard {
            Some(Wizard::Create(w)) => {
                w.start_share_gather(0, at.saturating_sub(1), of);
            }
            Some(Wizard::Backup(b)) => {
                let mut g =
                    CreateWizard::gathering(source, count, osk_bip::bip39::Language::English, 0, 0);
                g.set_device(has_camera, tier_d);
                g.start_share_gather(0, at.saturating_sub(1), of);
                b.set_gather(g);
            }
            _ => return,
        }
        self.sync_create();
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The gatherer has reached its words, which are one run's bytes.
    fn codex32_gathered(&mut self) {
        let taken = match &mut self.wizard {
            Some(Wizard::Create(w)) => {
                let bytes = w.mnemonic().map(entropy_bytes);
                w.end_share_gather();
                w.go(create::Step::Codex32);
                bytes
            }
            Some(Wizard::Backup(b)) => {
                let bytes = b
                    .gather()
                    .and_then(CreateWizard::mnemonic)
                    .map(entropy_bytes);
                b.drop_gather();
                bytes
            }
            _ => None,
        };
        let Some((mut buf, n)) = taken else {
            self.cancel_wizard();
            return;
        };
        let next = self.codex32.take_random(&buf[..n]);
        buf.zeroize();
        self.sync_create();
        self.after_codex32(next);
    }

    /// The chevron inside the gatherer: its own steps first, then the
    /// plan, with every byte gathered so far forgotten.
    fn codex32_gather_back(&mut self) {
        let leaving = match &mut self.wizard {
            Some(Wizard::Create(w)) => {
                let more = w.back();
                if !more {
                    w.end_share_gather();
                    w.go(create::Step::Codex32);
                }
                !more
            }
            Some(Wizard::Backup(b)) => match b.gather_mut() {
                Some(g) => {
                    let more = g.back();
                    if !more {
                        b.drop_gather();
                    }
                    !more
                }
                None => true,
            },
            _ => true,
        };
        if leaving && !self.codex32.back() {
            self.cancel_wizard();
            return;
        }
        self.sync_create();
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// Every random byte is in: the strings themselves.
    fn make_codex32(&mut self) {
        if !self.codex32.split() {
            self.cancel_wizard();
            return;
        }
        if let Some(Wizard::Create(w)) = &mut self.wizard {
            w.go(create::Step::Codex32);
        }
        self.sync_create();
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The last string has been shown, and the Result behind it where
    /// the flow has one: Create goes on to Confirm, Backup closes.
    fn codex32_finished(&mut self) {
        let verified = self.codex32.verified();
        let network = self.network;
        match &mut self.wizard {
            Some(Wizard::Create(w)) => {
                if !w.codex32_made(verified, network) {
                    self.cancel_wizard();
                    return;
                }
                self.ui.set_scroll(ids::SCROLL, 0);
            }
            Some(Wizard::Backup(b)) => {
                let key = b.key();
                if let Some(k) = self.keys.get_mut(key) {
                    k.backup_verified = verified;
                }
                self.cancel_wizard();
                self.sync_kept();
            }
            _ => {}
        }
    }

    // ----- the Create wizard's camera (`docs/PLANNING.md` §8.1 item 4) -----

    /// The camera-noise step's preview, for the viewfinder.
    pub(crate) fn camera_preview(&self) -> Option<osk_ui::components::Preview> {
        self.create_preview
            .as_ref()
            .map(|p| osk_ui::components::Preview {
                width: p.width(),
                height: p.height(),
                pixels: p.shared(),
                chroma: p.shared_chroma(),
            })
    }

    /// The wizard gathering entropy: Create a key's own, or the one a
    /// Seed XOR split runs once per random part (`docs/PLANNING.md`
    /// §16.92). The camera, the entropy request and the entry screens
    /// are the same in both, so they are reached the same way.
    pub(crate) fn gatherer(&self) -> Option<&CreateWizard> {
        match &self.wizard {
            Some(Wizard::Create(w)) => Some(w),
            Some(Wizard::Backup(b)) => b.gather(),
            _ => None,
        }
    }

    fn gatherer_mut(&mut self) -> Option<&mut CreateWizard> {
        match &mut self.wizard {
            Some(Wizard::Create(w)) => Some(w),
            Some(Wizard::Backup(b)) => b.gather_mut(),
            _ => None,
        }
    }

    /// Whether `source` can be chosen on this build: the camera row
    /// needs a camera, and the device row is not offered on Tier D.
    pub(crate) fn source_available(&self, source: osk_entropy::Source) -> bool {
        match source {
            osk_entropy::Source::Camera => self.has_camera,
            osk_entropy::Source::Device => self.tier != AssuranceTier::D,
            _ => true,
        }
    }

    /// Turns the camera on and off as the Create wizard enters and
    /// leaves its camera step, wipes the frames it held, and asks the
    /// shell for its own randomness when the device step opens.
    fn sync_create(&mut self) {
        let wanted = self.gatherer().is_some_and(CreateWizard::wants_camera);
        if wanted != self.create_camera {
            self.create_camera = wanted;
            if wanted {
                self.commands.push_back(Command::CameraOn);
            } else {
                self.commands.push_back(Command::CameraOff);
                self.create_preview = None;
                self.create_frame = None;
            }
        }
        let asking = self
            .gatherer()
            .is_some_and(|w| w.step() == create::Step::Device && !w.device_ready());
        if asking && !self.create_entropy_asked {
            self.commands.push_back(Command::RequestEntropy);
        }
        self.create_entropy_asked = asking;
    }

    /// The shutter: the frame on screen is the one that is hashed.
    fn create_shutter(&mut self) {
        let now = self.now_ms;
        let Some(luma) = self.create_frame.take() else {
            return;
        };
        if let Some(w) = self.gatherer_mut() {
            w.take_frame(&luma, now);
        }
    }

    /// The shell answered an entropy request while the Create wizard was
    /// waiting for one. Returns whether it was waiting.
    fn create_entropy(&mut self, bytes: &[u8; 32]) -> bool {
        let Some(w) = self.gatherer_mut() else {
            return false;
        };
        if w.step() != create::Step::Device || w.device_ready() {
            return false;
        }
        w.set_device_entropy(bytes);
        true
    }

    fn start_backup(&mut self, key: usize, step: BackupStep) {
        // Every other step of the flow shows or checks the words. The
        // encrypted backup is the one that does not: a key with no
        // words is sealed as its master seed (§16.112 rule 1).
        let words = self.keys.get(key).is_some_and(LoadedKey::has_mnemonic);
        let seed_only = step == BackupStep::Passphrase && self.keys.get(key).is_some();
        if words || seed_only {
            self.wizard = Some(Wizard::Backup(BackupFlow::new(key, step)));
            self.ui.set_scroll(ids::SCROLL, 0);
        }
    }

    // ----- scanning and routing (UX.md §4) -----

    /// Opens the scanner for `expect` and asks the shell for the camera.
    fn start_scan(&mut self, expect: Expect) {
        self.push(Screen::Scan);
        self.scan = Some(ScanState::new(expect));
        self.commands.push_back(Command::CameraOn);
    }

    /// The scanner's "Type" row: the keyboard the flow behind it has
    /// for the value (PLANNING §16.88). The scanner is left first, so
    /// the entry stands where the scanner stood.
    fn scan_type(&mut self, expect: Expect) {
        let from_explore = self.stack.last() == Some(&Screen::Explore);
        match expect {
            // The Create wizard's part entry is behind this scanner.
            Expect::Seed
                if matches!(&self.wizard, Some(Wizard::Create(w))
                    if w.step() == create::Step::XorPart) =>
            {
                self.pop_screen();
            }
            Expect::Seed if from_explore => {
                self.pop_screen();
                self.start_explore_entry();
            }
            // Load's own word entry, at the count step the typed source
            // opens on.
            Expect::Seed => {
                self.pop_screen();
                let mut w = LoadWizard::new();
                w.go(Step::Count);
                self.wizard = Some(Wizard::Load(w));
                self.ui.set_scroll(ids::SCROLL, 0);
            }
            Expect::Address => {
                self.pop_screen();
                self.push(Screen::Verify);
                self.ui.set_modifiers(keyboard::Modifiers::default());
                self.address_layer();
            }
            Expect::Message => {
                self.pop_screen();
                self.push(Screen::SignMessage);
                self.message = Some(message::SignMessage::typing());
            }
            // The builder is behind the scanner, so leaving it puts the
            // wizard back on screen at the entry it opens.
            Expect::Cosigner => {
                self.pop_screen();
                self.with_build(|w| w.type_key());
            }
            _ => {
                if let Some(tool) = expect.tool() {
                    self.pop_screen();
                    self.push(Screen::Tool(tool));
                }
            }
        }
    }

    /// Convert key's "Use a loaded key": the chooser every flow that
    /// picks among the loaded keys uses, feeding the chosen key's
    /// account xpub into the field.
    fn open_convert_key_choice(&mut self) {
        if self.keys.is_empty() {
            return;
        }
        self.overlay = Some(Overlay::Choice(Picker::ConvertKey(self.keys.len()), 0));
    }

    /// Drops the scanner, turning the camera off if it was on.
    fn leave_scan(&mut self) {
        if let Some(s) = self.scan.take()
            && s.camera_on()
        {
            self.commands.push_back(Command::CameraOff);
        }
    }

    /// A frame from the shell: redraw the preview, and nothing else
    /// (§4.9). The codes in it are the shell's to read, and come back
    /// as [`Event::Scanned`].
    fn camera_frame(&mut self, width: u16, height: u16, luma: Vec<u8>, chroma: Option<Vec<u8>>) {
        if self.create_camera {
            self.create_frame(width, height, luma);
            return;
        }
        if self.scan_stage() != Some(ScanStage::Camera) {
            return;
        }
        // The frame may picture a SeedQR: wipe it once drawn. The
        // chroma is part of the same picture and goes the same way.
        let luma = Zeroizing::new(luma);
        // A chroma plane that is not the one this luma calls for says
        // nothing about the frame's colour, so the preview is grey
        // rather than guessed at.
        let (cw, ch) = scan::chroma_size(usize::from(width), usize::from(height));
        let chroma = chroma
            .filter(|uv| uv.len() >= cw * ch * 2)
            .map(Zeroizing::new);
        // The camera on a device someone built is mounted whichever way
        // its case allows, so the setting turns the frame upright
        // before the preview is made of it (`docs/DESIGN.md` §4.9).
        let rotation = self.camera_rotation();
        let (width, height, luma, chroma) = if rotation == CameraRotation::Deg0 {
            (width, height, luma, chroma)
        } else {
            let turned = chroma.as_ref().map(|uv| {
                Zeroizing::new(scan::rotate_chroma(
                    usize::from(width),
                    usize::from(height),
                    uv,
                    rotation,
                ))
            });
            let (w, h, px) = scan::rotate(usize::from(width), usize::from(height), &luma, rotation);
            (w, h, Zeroizing::new(px), turned)
        };
        // The square shows the frame whether or not anything decodes.
        let (w, h) = (usize::from(width), usize::from(height));
        let factor = scan::reduce_factor(w, h);
        if let Some(s) = self.scan.as_mut() {
            let (out_w, out_h, pixels) = scan::reduce(w, h, &luma);
            let uv = chroma
                .as_ref()
                .map(|uv| scan::reduce_chroma(w, h, uv, factor));
            s.set_preview(out_w, out_h, pixels, uv);
        }
        self.render();
    }

    /// A frame for the Create wizard's camera step: the preview the
    /// viewfinder draws, and the full luma the shutter hashes. Both are
    /// wiped when the step is left, as a scanner's frame is.
    fn create_frame(&mut self, width: u16, height: u16, luma: Vec<u8>) {
        let luma = Zeroizing::new(luma);
        let (w, h) = (usize::from(width), usize::from(height));
        let (out_w, out_h, pixels) = scan::reduce(w, h, &luma);
        self.create_preview = Some(scan::Preview::new(out_w, out_h, pixels, None));
        self.create_frame = Some(luma);
        self.render();
    }

    /// A code the shell read from its camera's frames (§4.9). It is
    /// routed exactly as a code the core used to find itself, and is
    /// ignored off the scan screen, as a frame is.
    fn scanned(&mut self, bytes: Vec<u8>) {
        if self.scan_stage() != Some(ScanStage::Camera) {
            return;
        }
        // A payload may be a SeedQR: it is a secret until it is routed.
        self.route_payload(Zeroizing::new(bytes), true);
        self.ui.set_scroll(ids::SCROLL, 0);
        self.render();
    }

    /// The shell has no camera: offer the file channel.
    fn camera_unavailable(&mut self) {
        // The shell says this once; the Load wizard's source list dims
        // its scan row from then on, with the reason beside it (§4.11).
        self.has_camera = false;
        if self.screen == Screen::Scan
            && let Some(s) = self.scan.as_mut()
        {
            s.unavailable();
        }
        self.render();
    }

    /// Routes a payload by kind. `allow_ur` is off for the bytes a
    /// `ur:bytes` carried, so a UR cannot nest.
    fn route_payload(&mut self, bytes: Zeroizing<Vec<u8>>, allow_ur: bool) {
        let s = self.strings();
        let expect = self.scan.as_ref().map(ScanState::expect);
        let tool = expect.and_then(Expect::tool);
        // §4.10: a secret is never pasted, and no calculator is a place
        // to put one. Words in the clear, a SeedQR, an extended private
        // key or a private key is refused here rather than reaching the
        // screen it was aimed at.
        let pasted = self.scan.as_ref().is_some_and(ScanState::pasted);
        if (pasted || tool.is_some()) && is_secret_payload(&bytes) {
            self.scan_failed(String::from(s.scan_reason_secret));
            return;
        }
        // The five calculators take a string: the payload becomes what
        // is typed, and the answer opens where the tool can work one
        // out.
        if let Some(tool) = tool {
            match core::str::from_utf8(bytes.trim_ascii()) {
                Ok(text) => {
                    let text = String::from(text);
                    self.tool_scanned(tool, text);
                }
                Err(_) => self.scan_failed(String::from(s.scan_reason_type)),
            }
            return;
        }
        // The two message expectations take any text at all: a message
        // is whatever a person wants signed, and a signed message is
        // three lines no classifier knows. Nothing routes this way
        // unless the scanner was opened for it, so a PSBT is still a
        // PSBT everywhere else (UX.md §4).
        match self.scan.as_ref().map(ScanState::expect) {
            Some(Expect::Message) => {
                if let Ok(text) = core::str::from_utf8(bytes.trim_ascii()) {
                    let text = String::from(text);
                    self.message_scanned(text);
                    return;
                }
            }
            // A cosigner's key is read as text: the same notation a
            // wallet arrives in, one key of it.
            Some(Expect::Cosigner) => {
                match core::str::from_utf8(bytes.trim_ascii()) {
                    Ok(text) => {
                        let text = String::from(text);
                        self.cosigner_scanned(&text);
                    }
                    Err(_) => self.scan_failed(String::from(s.scan_reason_type)),
                }
                return;
            }
            // A transaction to read is a PSBT the way Sign takes one, or
            // a raw transaction in hex or in bytes. A UR is left to the
            // assembler, which routes the message it completes back
            // here.
            Some(Expect::Transaction)
                if !matches!(classify(&bytes), PayloadKind::Ur { .. }) || !allow_ur =>
            {
                self.transaction_scanned(&bytes);
                return;
            }
            // Compare transactions takes a PSBT the way Sign does, and
            // what it does with it is decided by whether the first one
            // has already arrived (§16.111 rule 4).
            Some(Expect::CompareTransaction)
                if !matches!(classify(&bytes), PayloadKind::Ur { .. }) || !allow_ur =>
            {
                self.compare_scanned(&bytes);
                return;
            }
            // A note read from a file is the file's own bytes as
            // UTF-8; an `osk-backup` holding one asks the passphrase
            // first, and falls through to the routing below
            // (§16.112 rule 2).
            // §16.113: a transaction to check against a silent
            // payments wallet, and the previous transaction an input
            // of it needs, are read as bytes and never routed as
            // anything else.
            Some(expect @ (Expect::SilentPayment | Expect::SilentPrevious))
                if !matches!(classify(&bytes), PayloadKind::Ur { .. }) || !allow_ur =>
            {
                self.silent_scanned(&bytes, expect == Expect::SilentPrevious);
                return;
            }
            Some(Expect::Note) if !matches!(classify(&bytes), PayloadKind::EncryptedBackup) => {
                self.note_opened(&bytes);
                return;
            }
            Some(Expect::SignedMessage) => {
                match core::str::from_utf8(&bytes)
                    .ok()
                    .and_then(osk_psbt::message::parse_signed)
                {
                    Some(signed) => self.signed_message_scanned(signed),
                    None => self.scan_failed(String::from(s.msg_not_signed)),
                }
                return;
            }
            _ => {}
        }
        let text = || String::from_utf8_lossy(bytes.trim_ascii()).into_owned();
        match classify(&bytes) {
            PayloadKind::SeedQr { .. } => {
                match seedqr::from_digits(bytes.trim_ascii(), Language::English) {
                    Ok(m) => self.load_scanned(&m),
                    Err(_) => self.scan_failed(String::from(s.scan_reason_seedqr)),
                }
            }
            // `classify` matched the 16 or 32 bytes `from_entropy` asks
            // for, so there is no failure left for a reason to name.
            PayloadKind::CompactSeedQr { .. } => {
                if let Ok(m) = seedqr::from_entropy(&bytes, Language::English) {
                    self.load_scanned(&m);
                }
            }
            // A backup the magic names but the header refuses is
            // refused here with a reason, rather than being routed as
            // something it is not.
            PayloadKind::EncryptedBackup => match osk_backup::oskb::read_header(&bytes) {
                Ok(_) => self.backup_scanned(bytes.to_vec()),
                Err(osk_backup::oskb::Error::Memory(mib)) => {
                    let reason = strings::fill1(s.backup_needs_memory, &alloc::format!("{mib}"));
                    self.scan_failed(reason);
                }
                Err(_) => self.scan_failed(String::from(s.scan_reason_backup)),
            },
            // A carry file is a partly signed threshold transaction and
            // is read into Sign as a PSBT is (§16.103).
            PayloadKind::Psbt | PayloadKind::ThresholdCarry => self.sign_scanned(&bytes),
            PayloadKind::Ur { .. } if allow_ur => self.route_ur(&text()),
            PayloadKind::Address => self.verify_scanned(&text()),
            // §16.113: a silent payment address is no address of any
            // wallet's chain, so Check an address says what it is and
            // shows it whole rather than searching for it.
            PayloadKind::SilentAddress => self.verify_scanned(&text()),
            PayloadKind::Descriptor { checksum } => self.descriptor_scanned(text(), checksum),
            PayloadKind::Xpub => self.xpub_scanned(text()),
            PayloadKind::MultisigConfig => self.multisig_config_scanned(&text()),
            PayloadKind::ColdcardXpubs => self.coldcard_xpubs_scanned(&text()),
            PayloadKind::Bip39Words { language, .. } => self.words_scanned(&text(), language),
            // A codex32 string is a key arriving: it goes to the entry
            // that types one, filled, and is checked there (§16.109
            // rule 3).
            PayloadKind::Codex32 => self.codex32_scanned(&text()),
            // Round 2 of a Bitcoin Secure Multisig Setup is the wallet
            // arriving, with the rows BIP 129 asks every signer to
            // check before registering it.
            PayloadKind::BsmsDescriptor => self.bsms_descriptor_scanned(&text()),
            // Round 1 is one signer's key, which is not a wallet and is
            // refused by what it is.
            PayloadKind::BsmsSigner => {
                self.scan_failed(String::from(s.scan_reason_bsms_signer));
            }
            PayloadKind::Ur { .. } | PayloadKind::Unknown => {
                if let Some(s) = self.scan.as_mut() {
                    s.set_unknown(bytes.to_vec());
                }
            }
        }
    }

    /// One part of a UR (or a whole single-part one).
    fn route_ur(&mut self, text: &str) {
        let Some(s) = self.scan.as_mut() else {
            return;
        };
        let mut received = s.ur().receive(text);
        if received == Err(ur::Error::Mismatch) {
            // A different message: start over with this part.
            s.reset_ur();
            received = s.ur().receive(text);
        }
        match received {
            Ok(false) => {}
            Ok(true) => {
                let message = s.ur().message().cloned();
                s.reset_ur();
                match message {
                    Some(ur::Message::Psbt(psbt)) => self.sign_scanned(&psbt),
                    Some(ur::Message::Bytes(b)) => self.route_payload(Zeroizing::new(b), false),
                    Some(ur::Message::Other { .. }) => {
                        let reason = String::from(self.strings().scan_reason_type);
                        self.scan_failed(reason);
                    }
                    None => {}
                }
            }
            Err(_) => {
                s.reset_ur();
                let reason = String::from(self.strings().scan_reason_type);
                self.scan_failed(reason);
            }
        }
    }

    fn scan_failed(&mut self, error: String) {
        if let Some(s) = self.scan.as_mut() {
            s.set_error(error);
        }
    }

    /// A SeedQR: the Load wizard at its language step, the Explore
    /// workspace's own checksum step where the scanner was opened from
    /// Explore, or one Seed XOR part where the Create wizard is
    /// collecting them.
    fn load_scanned(&mut self, m: &Mnemonic) {
        if self.xor_part_scanned(m) {
            return;
        }
        self.words_to_wizard(LoadWizard::from_words(m.indices(), m.language()));
    }

    /// Where a wizard built from scanned words goes. Explore keeps its
    /// own: the words go no further than the workspace, and the key is
    /// never added.
    fn words_to_wizard(&mut self, mut wizard: LoadWizard) {
        if self.stack.last() == Some(&Screen::Explore) {
            self.pop_screen();
            self.explore_entry = true;
            wizard.go(Step::Checksum);
            self.wizard = Some(Wizard::Load(wizard));
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        self.go_home();
        self.wizard = Some(Wizard::Load(wizard));
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// A codex32 string, scanned, pasted or typed into the scanner: the
    /// Load wizard opens on the entry that types one with the string
    /// already in the field and already checked, so a secret lands on
    /// the confirmation and a share starts the gathering
    /// (`docs/PLANNING.md` §16.109 rule 3).
    fn codex32_scanned(&mut self, text: &str) {
        let Some(wizard) = LoadWizard::from_codex32(text.trim(), self.network) else {
            let reason = String::from(self.strings().scan_reason_type);
            self.scan_failed(reason);
            return;
        };
        self.go_home();
        self.wizard = Some(Wizard::Load(wizard));
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// One Seed XOR part read as a seed code: it fills the part on
    /// screen and the wizard moves on to the next. Returns whether the
    /// wizard was collecting parts.
    fn xor_part_scanned(&mut self, m: &Mnemonic) -> bool {
        let collecting = matches!(&self.wizard, Some(Wizard::Create(w))
            if w.step() == create::Step::XorPart);
        if !collecting {
            return false;
        }
        let network = self.network;
        self.pop_screen();
        if let Some(Wizard::Create(w)) = &mut self.wizard {
            w.set_part_words(m.indices(), m.language(), network);
        }
        self.ui.set_scroll(ids::SCROLL, 0);
        true
    }

    /// Plain-text words: the caution first, then the Load wizard.
    fn words_scanned(&mut self, text: &str, language: Language) {
        let mut indices = Zeroizing::new([0u16; MAX_WORDS]);
        let mut n = 0;
        for w in text.split_whitespace().take(MAX_WORDS) {
            let Some(i) = osk_codec::classify::word_index(language, w) else {
                return;
            };
            indices[n] = i;
            n += 1;
        }
        let mut wizard = LoadWizard::from_words(&indices[..n], language);
        // The words name their language. Ask only when more than one
        // list holds every one of them (UX review 2026-09-07, Q4).
        let matches = Language::ALL
            .into_iter()
            .filter(|lang| {
                text.split_whitespace()
                    .all(|w| osk_codec::classify::word_index(*lang, w).is_some())
            })
            .count();
        if matches > 1 {
            wizard.go(Step::Language);
        }
        match self.scan.as_mut() {
            Some(s) => s.set_pending_words(wizard),
            None => {
                self.go_home();
                self.wizard = Some(Wizard::Load(wizard));
            }
        }
    }

    /// A PSBT: the Sign flow at its summary.
    fn sign_scanned(&mut self, bytes: &[u8]) {
        self.go_home();
        self.push(Screen::Sign);
        let refs = self.key_refs();
        let shares = self.share_refs();
        let network = self.network;
        let wallets = self.wallets.clone();
        let view = self.musig_session.as_ref().map(|s| s.view());
        self.sign
            .load(bytes, refs, wallets, shares, network, view.as_ref());
        self.check_transaction_signatures(false);
    }

    /// One of the two transactions Compare reads (`docs/PLANNING.md`
    /// §16.111 rule 4): the first is kept and the tool asks for the
    /// second, the second is compared with it and the Result says what
    /// differs.
    fn compare_scanned(&mut self, bytes: &[u8]) {
        let psbt = match sign::parse_psbt(bytes) {
            Ok(psbt) => psbt,
            Err(_) => {
                let reason = String::from(self.strings().sign_not_psbt);
                self.scan_failed(reason);
                return;
            }
        };
        // The tool travels with the scan: leaving a screen drops what
        // it held, and the way to this screen goes through Home.
        let mut tool = core::mem::take(&mut self.compare_tx);
        if tool.has_first() {
            tool.compare_with(&psbt);
        } else {
            tool.set_first(psbt);
        }
        self.go_home();
        self.push(Screen::CompareTx);
        self.compare_tx = tool;
    }

    /// A transaction to read rather than sign (Tools › Decode a
    /// transaction): a PSBT, or a raw transaction wrapped in one so
    /// that the same inspection and the same review screens serve both.
    /// Whatever keys and wallets this device holds are the context, so
    /// change of a wallet in use is recognised here as it is in Sign;
    /// nothing is signed either way.
    fn transaction_scanned(&mut self, bytes: &[u8]) {
        let network = self.network;
        let refs = self.key_refs();
        let shares = self.share_refs();
        let wallets = self.wallets.clone();
        if sign::parse_psbt(bytes).is_ok() {
            self.go_home();
            self.push(Screen::Decode);
            let view = self.musig_session.as_ref().map(|s| s.view());
            self.decode
                .read(bytes, refs, wallets, shares, network, view.as_ref());
            self.check_transaction_signatures(true);
            return;
        }
        match osk_psbt::transaction::wrap_raw(bytes) {
            Some((wrapper, txid)) => {
                self.go_home();
                self.push(Screen::Decode);
                let view = self.musig_session.as_ref().map(|s| s.view());
                self.decode.read_raw(
                    &wrapper,
                    txid,
                    refs,
                    wallets,
                    shares,
                    network,
                    view.as_ref(),
                );
                self.check_transaction_signatures(true);
            }
            // Neither a PSBT nor a transaction: the scanner says so and
            // stays where it is.
            None => {
                let reason = String::from(self.strings().sign_not_psbt);
                self.scan_failed(reason);
            }
        }
    }

    /// A string a calculator can work from: it becomes the field, and
    /// the answer opens where the tool can work one out. Where it
    /// cannot, the Type entry opens with the text in it and the inline
    /// error under the field, so a person sees what was wrong.
    fn tool_scanned(&mut self, tool: Tool, text: String) {
        self.tool_scanned_as(tool, text, None);
    }

    /// The same, forcing how the Hashes field is read: an unknown code's
    /// bytes reach it as hex, whatever they spell.
    fn tool_scanned_as(
        &mut self,
        tool: Tool,
        text: String,
        read_as: Option<osk_codec::encodings::ReadAs>,
    ) {
        self.pop_screen();
        self.push(Screen::Tool(tool));
        if let Some(read_as) = read_as {
            self.calc.set_read_as(read_as);
        }
        self.calc.set(tool, text);
        if self
            .calc
            .ready(tool, self.network, &self.policy_key_expressions())
        {
            self.push(Screen::ToolResult(tool));
        }
    }

    /// An address: Verify with its result.
    fn verify_scanned(&mut self, text: &str) {
        let result = self.check_address(text);
        self.go_home();
        self.push(Screen::Verify);
        self.verify.set_result(text, result);
    }

    /// A text to sign: the message screen over Home. Nothing arrives
    /// this way without a key, since the row that opens the scanner is
    /// dead without one.
    fn message_scanned(&mut self, text: String) {
        if self.keys.is_empty() {
            return;
        }
        self.go_home();
        self.push(Screen::SignMessage);
        self.message = Some(message::SignMessage::new(text));
    }

    /// A signed message: the answer over Home.
    fn signed_message_scanned(&mut self, signed: osk_psbt::message::SignedText) {
        let checked = self.check_signed(&signed);
        self.go_home();
        self.push(Screen::CheckedMessage);
        self.check = Some(message::CheckMessage::new(signed, checked));
    }

    /// Whether a signed message holds. An address of another network is
    /// not one this device checks against, so it is not valid here.
    fn check_signed(
        &self,
        signed: &osk_psbt::message::SignedText,
    ) -> Option<osk_psbt::message::Checked> {
        let address = osk_bip::address::parse(&signed.address, self.network).ok()?;
        osk_psbt::message::verify(&address, &signed.message, &signed.signature).ok()
    }

    /// Derives the address the message screen names, if it is not
    /// cached yet: the first receive address of the chosen script type.
    fn prepare_message(&mut self) {
        let Some((key, script)) = self.message.as_ref().map(|f| (f.key(), f.script())) else {
            return;
        };
        if self.account(key, script).is_none() {
            return;
        }
        let Some(fp) = self.keys.get(key).map(|k| k.fingerprint) else {
            return;
        };
        if let Some(entry) = self
            .cache
            .iter_mut()
            .find(|c| c.fingerprint == fp && c.script == script)
            && entry.receive.is_empty()
            && let Ok(a) = entry.account.address(false, 0)
        {
            entry.receive.push(alloc::format!("{a}"));
        }
    }

    /// The address `flow` signs for.
    pub(crate) fn message_address(&self, flow: &message::SignMessage) -> String {
        let Some(fp) = self.keys.get(flow.key()).map(|k| k.fingerprint) else {
            return String::new();
        };
        self.cached_addresses(fp, flow.script(), false)
            .first()
            .cloned()
            .unwrap_or_default()
    }

    /// The hold on the message screen: the key signs the text for the
    /// address the screen names. A key that cannot derive that address
    /// signs nothing and the hold leaves the screen as it was.
    fn sign_message(&mut self) {
        let Some(flow) = self.message.as_ref() else {
            return;
        };
        let (key, script, format) = (flow.key(), flow.script(), flow.format());
        let text = String::from(flow.text());
        let network = self.network;
        let nonce = self.nonce;
        let aux = self.schnorr_aux();
        let signed = self
            .keys
            .get(key)
            .and_then(|k| k.master.as_ref())
            .and_then(|master| {
                let path = master.account_path(script, 0).ok()?;
                let zero = osk_bip::keys::ChildNumber::from_normal_idx(0).ok()?;
                let derived = master.derive(&path.extend([zero, zero]));
                osk_psbt::message::sign(
                    master.secp(),
                    derived.secret_key(),
                    script,
                    network,
                    &text,
                    format,
                    nonce,
                    aux,
                )
                .ok()
            });
        if let (Some(flow), Some(signed)) = (self.message.as_mut(), signed) {
            flow.signed(message::Outcome {
                address: signed.address,
                signature: signed.signature,
                format: signed.format,
                save: sign::Save::Idle,
            });
        }
    }

    fn tap_sign_message(&mut self, id: Id) {
        let Some(flow) = self.message.as_ref() else {
            return;
        };
        let stage = flow.stage();
        let address = self.message_address(flow);
        match stage {
            // The field's one control is its keyboard.
            message::Stage::Typing => {}
            message::Stage::Message => {
                if id == ids::MSG_KEY && self.keys.len() > 1 {
                    let picked = flow.key();
                    self.overlay =
                        Some(Overlay::Choice(Picker::MessageKey(self.keys.len()), picked));
                } else if id == ids::MSG_SCRIPT {
                    self.open_script_choice(flow.script());
                } else if id == ids::MSG_ADDRESS {
                    self.open_compare(self.strings().row_address, address);
                } else if id == ids::MSG_FORMAT && flow.format_offered() {
                    let picked = message::FORMATS
                        .iter()
                        .position(|f| *f == flow.format())
                        .unwrap_or(0);
                    self.overlay = Some(Overlay::Choice(Picker::MessageFormat, picked));
                } else if id == ids::MSG_CONTINUE {
                    if let Some(flow) = self.message.as_mut() {
                        flow.confirm();
                    }
                    self.ui.set_scroll(ids::SCROLL, 0);
                }
            }
            message::Stage::Confirm => {
                if id == ids::MSG_ADDRESS {
                    self.open_compare(self.strings().row_address, address);
                }
            }
            message::Stage::Result => {
                let Some(out) = flow.outcome() else {
                    return;
                };
                let (saved, signature, signed_for) =
                    (out.save, out.signature.clone(), out.address.clone());
                if id == ids::MSG_ADDRESS {
                    self.open_compare(self.strings().row_address, signed_for);
                } else if id == ids::MSG_SIGNATURE {
                    self.open_compare(self.strings().sign_signature, signature);
                } else if id == ids::MSG_SAVE && saved != sign::Save::Waiting {
                    if let Some(bytes) = flow.file_bytes() {
                        self.commands.push_back(Command::WriteFile {
                            kind: FileKind::Any,
                            name_hint: String::from(message::SIGNATURE_FILE),
                            bytes,
                        });
                        if let Some(flow) = self.message.as_mut() {
                            flow.mark_saving();
                        }
                    }
                } else if id == ids::MSG_QR {
                    if let Some(flow) = self.message.as_mut() {
                        flow.show_qr();
                    }
                    self.ui.set_scroll(ids::SCROLL, 0);
                } else if id == ids::MSG_DONE {
                    self.go_home();
                }
            }
            message::Stage::Qr => {}
        }
    }

    fn tap_checked_message(&mut self, id: Id) {
        let Some(check) = self.check.as_ref() else {
            return;
        };
        if id == ids::MSG_ADDRESS {
            let address = String::from(check.address());
            self.open_compare(self.strings().row_address, address);
        } else if id == ids::MSG_READ {
            if let Some(check) = self.check.as_mut() {
                check.read();
            }
            self.ui.set_scroll(ids::SCROLL, 0);
        } else if id == ids::MSG_DONE {
            self.go_home();
        }
    }

    fn inspect_scanned(&mut self, doc: InspectDoc) {
        self.go_home();
        self.push(Screen::Inspect);
        self.inspect = Some(doc);
    }

    /// A scanned descriptor. Any descriptor a wallet policy can be built
    /// from — a multisig one, a BIP-388 policy, or the single-key
    /// `wpkh([fp/path]xpub/<0;1>/*)` a watch-only wallet arrives as — is
    /// a wallet being offered and is reviewed as one. A policy this
    /// device cannot read or whose checksum fails is refused where the
    /// code was read (§4.11); a descriptor that is no wallet at all is
    /// still a document to read.
    fn descriptor_scanned(&mut self, text: String, checksum: Option<bool>) {
        let s = self.strings();
        let policy = WalletPolicy::parse_any(&text);
        let refused = match &policy {
            Ok(policy) => {
                self.review_wallet(policy.clone());
                return;
            }
            // A wallet shape this device will not use — a key with one
            // chain, a repeated key, a placeholder out of order, a
            // threshold no script can hold — is refused where the code
            // was read (§4.11). `Template` means it is no wallet at
            // all, and a broken checksum has a row of its own that says
            // so; both of those are still documents to read.
            Err(error) => matches!(
                error,
                osk_bip::policy::Error::Derivation
                    | osk_bip::policy::Error::Key
                    | osk_bip::policy::Error::Placeholder
                    | osk_bip::policy::Error::Threshold
                    | osk_bip::policy::Error::Share
                    | osk_bip::policy::Error::Group
                    | osk_bip::policy::Error::Xpub
            ),
        };
        // A group record that does not hold together is a wallet this
        // device will not use, whatever else its text might be read as.
        if refused
            || text.contains('@')
            || text.contains("multi(")
            || text.contains("musig(")
            || osk_bip::threshold::ThresholdRecord::looks_like_record(&text)
        {
            self.scan_failed(String::from(s.scan_reason_policy));
            return;
        }
        self.inspect_scanned(InspectDoc {
            title: s.inspect_descriptor,
            text,
            descriptor: true,
            checksum,
            policy: None,
            name: None,
            from_menu: false,
            swap: None,
            bsms: None,
        });
    }

    /// BIP 129's descriptor record: the wallet it carries, at the same
    /// review every other wallet arrives at, with the paths it restricts
    /// derivation to, its first receive address and this device's place
    /// in the key list stated above the descriptor.
    fn bsms_descriptor_scanned(&mut self, text: &str) {
        let s = self.strings();
        let record = match bsms::DescriptorRecord::parse(text) {
            Ok(record) => record,
            Err(bsms::Error::Encrypted) => {
                self.scan_failed(String::from(s.scan_reason_bsms_encrypted));
                return;
            }
            Err(_) => {
                self.scan_failed(String::from(s.scan_reason_bsms));
                return;
            }
        };
        let ours = record.policy.keys().iter().position(|k| {
            k.fingerprint()
                .is_some_and(|fp| self.keys.iter().any(|key| key.fingerprint == fp))
        });
        let paths = if record.paths.is_empty() {
            String::from(s.value_none)
        } else {
            record.paths.join(",")
        };
        let facts = inspect::Bsms {
            paths,
            first_address: record.first_address.clone(),
            ours,
        };
        let mut doc = self.wallet_doc(record.policy, false);
        doc.bsms = Some(facts);
        self.inspect_scanned(doc);
    }

    /// An extended public key on its own. It is the wallet of one key,
    /// once the script type is known: a SLIP-132 spelling names it, and
    /// an `xpub` or `tpub` names none, so the Choice opens over the key's
    /// own document with SegWit checked and its Continue builds the
    /// wallet (§16.72).
    fn xpub_scanned(&mut self, text: String) {
        // `xpub` and `tpub` are BIP-32's own prefixes and name no script;
        // SLIP-132's four each name one.
        let script = match osk_bip::xkey::decode_xpub(&text) {
            Ok(_) => ScriptType::NativeSegwit,
            Err(_) => osk_bip::slip132::decode_xpub(&text)
                .map(|(_, script)| script)
                .unwrap_or(ScriptType::NativeSegwit),
        };
        let s = self.strings();
        self.inspect_scanned(InspectDoc {
            title: s.inspect_xpub,
            text,
            descriptor: false,
            checksum: None,
            policy: None,
            name: None,
            from_menu: false,
            swap: None,
            bsms: None,
        });
        let i = ScriptType::ALL
            .iter()
            .position(|t| *t == script)
            .expect("every script type is in the list");
        self.overlay = Some(Overlay::Choice(Picker::XpubScript, i));
    }

    /// The wallet the extended public key under the Choice makes at the
    /// script type that was picked.
    fn xpub_wallet(&mut self, script: ScriptType) {
        let Some(text) = self.inspect.as_ref().map(|d| d.text.clone()) else {
            return;
        };
        let xpub = osk_bip::xkey::decode_xpub(&text)
            .ok()
            .or_else(|| osk_bip::slip132::decode_xpub(&text).ok().map(|(x, _)| x));
        let Some(policy) = xpub.and_then(|x| WalletPolicy::from_xpub(x, script).ok()) else {
            return;
        };
        self.review_wallet(policy);
    }

    /// The review of one wallet: its own descriptor, so that the policy
    /// and the descriptor of the same wallet read the same.
    fn wallet_doc(&self, policy: WalletPolicy, from_menu: bool) -> InspectDoc {
        self.named_wallet_doc(policy, from_menu, None)
    }

    /// The same, for a wallet whose file named it.
    fn named_wallet_doc(
        &self,
        policy: WalletPolicy,
        from_menu: bool,
        name: Option<String>,
    ) -> InspectDoc {
        InspectDoc {
            title: self.strings().wallet_title,
            text: policy.to_descriptor_checksummed(),
            descriptor: true,
            checksum: Some(true),
            swap: self.wallet_swap(&policy),
            policy: Some(policy),
            name,
            from_menu,
            bsms: None,
        }
    }

    /// What a reviewed wallet is one key away from (UX.md E2): a wallet
    /// already in use, or, for a wallet of one key, a master this device
    /// holds.
    ///
    /// A wallet already in use is the wallet itself, not a swap of it.
    fn wallet_swap(&self, policy: &WalletPolicy) -> Option<Swap> {
        if self.wallet_in_use(policy) {
            return None;
        }
        self.claimed_key_swap(policy)
            .or_else(|| self.replaced_key_swap(policy))
    }

    /// A wallet in use of the same template and the same number of keys
    /// where exactly one key differs — by xpub, by origin, or both.
    ///
    /// Two or more differing keys is a different wallet, and a wallet of
    /// one key shares no key with any other, so both are left alone.
    fn replaced_key_swap(&self, policy: &WalletPolicy) -> Option<Swap> {
        let keys = policy.keys();
        if keys.len() < 2 {
            return None;
        }
        for wallet in &self.wallets {
            if wallet.template() != policy.template() || wallet.keys().len() != keys.len() {
                continue;
            }
            let mut differ = keys
                .iter()
                .zip(wallet.keys())
                .enumerate()
                .filter(|(_, (new, old))| new != old);
            let Some((i, (new, old))) = differ.next() else {
                continue;
            };
            if differ.next().is_some() {
                continue;
            }
            return Some(Swap {
                key: i,
                title: self.strings().inspect_swap,
                value: alloc::format!(
                    "{} \u{00b7} {} \u{00b7} {}",
                    self.policy_call_name(wallet),
                    wallet.checksum(),
                    self.key_change(old, new)
                ),
            });
        }
        None
    }

    /// Which key changed: the key that was there, and what stands in its
    /// place. A key whose fingerprint is the one it replaces is the swap
    /// this exists for, so the row says the fingerprint is the same.
    fn key_change(&self, old: &PolicyKey, new: &PolicyKey) -> String {
        let s = self.strings();
        let was = match old.fingerprint() {
            Some(fp) => text::fingerprint_hex(fp),
            None => String::from(s.wallet_origin_unknown),
        };
        if old.fingerprint().is_some() && old.fingerprint() == new.fingerprint() {
            return strings::fill1(s.inspect_swap_same, &was);
        }
        match new.fingerprint() {
            Some(fp) => strings::fill(s.inspect_swap_replaced, &[&was, &text::fingerprint_hex(fp)]),
            None => strings::fill1(s.inspect_swap_no_origin, &was),
        }
    }

    /// A wallet of one key whose origin names a master this device holds
    /// and whose key is not the one that master derives at that path.
    ///
    /// A key of a master this device does not hold, or one this device
    /// holds the public key of alone, states nothing to compare with.
    fn claimed_key_swap(&self, policy: &WalletPolicy) -> Option<Swap> {
        let [key] = policy.keys() else {
            return None;
        };
        let fingerprint = key.fingerprint()?;
        let path = key.path()?;
        let master = self
            .keys
            .iter()
            .find(|k| k.fingerprint == fingerprint)?
            .master
            .as_ref()?;
        let mine = master.derive(path).to_xpub();
        let claimed = key.xpub();
        if mine.public_key == claimed.public_key && mine.chain_code == claimed.chain_code {
            return None;
        }
        Some(Swap {
            key: 0,
            title: self.strings().sign_key_row,
            value: strings::fill1(
                self.strings().inspect_swap_claims,
                &text::fingerprint_hex(fingerprint),
            ),
        })
    }

    /// A coordinator's multisig config, read from a code or a file: the
    /// same wallet the BIP-388 policy of it would be, reviewed the same
    /// way, and refused where it was read when it cannot be built
    /// (§4.11).
    fn multisig_config_scanned(&mut self, text: &str) {
        match osk_bip::multisig_config::parse_named(text) {
            Ok((policy, name)) => self.review_named_wallet(policy, name),
            Err(_) => {
                let reason = String::from(self.strings().scan_reason_policy);
                self.scan_failed(reason);
            }
        }
    }

    /// Coldcard's JSON account export, which carries the master and the
    /// derivation of every account it offers, so the wallet it makes is
    /// reviewed like any other.
    fn coldcard_xpubs_scanned(&mut self, text: &str) {
        match osk_bip::coldcard::parse(text) {
            Ok(policy) => self.review_named_wallet(policy, osk_bip::coldcard::name(text)),
            Err(_) => {
                let reason = String::from(self.strings().scan_reason_policy);
                self.scan_failed(reason);
            }
        }
    }

    /// That review, reached from the scanner: the code was read from
    /// wherever the person was, so the review opens over Home.
    fn review_wallet(&mut self, policy: WalletPolicy) {
        let doc = self.wallet_doc(policy, false);
        self.inspect_scanned(doc);
    }

    /// The same, for a wallet whose file gave it a name.
    fn review_named_wallet(&mut self, policy: WalletPolicy, name: Option<String>) {
        let doc = self.named_wallet_doc(policy, false, name);
        self.inspect_scanned(doc);
    }

    /// The wallet menu, with Wallets behind it: a wallet is reached from
    /// its row there whether that row was tapped or a review just put
    /// the wallet in use.
    fn open_wallet(&mut self, wallet: usize) {
        self.stack = alloc::vec![Screen::Home, Screen::Wallets];
        self.screen = Screen::Wallet(WalletRef::Policy(wallet));
        self.entered();
    }

    /// The reviewed wallet, put in use. What a person does with it is
    /// its menu, so the review is over and Keys is behind the menu
    /// either way; a wallet already in use opens its own row.
    ///
    /// A review carrying a danger card is held rather than tapped
    /// (§2.7), so this is reached both ways.
    fn use_reviewed_wallet(&mut self) {
        let Some(policy) = self.inspect.as_ref().and_then(|d| d.policy.clone()) else {
            return;
        };
        // A coordinator config's `Name:` line, or an export's own name,
        // is the wallet's name from the moment it is in use.
        if let Some(name) = self.inspect.as_ref().and_then(|d| d.name.clone()) {
            self.names.set_checksum_name(&policy.checksum(), &name);
        }
        let wallet = match self.wallets.iter().position(|w| *w == policy) {
            Some(i) => i,
            None => {
                self.wallets.push(policy);
                self.wallets.len() - 1
            }
        };
        // The sheet's note belongs to the wallet, so a kept sheet whose
        // wallet is added again is a wallet whose Recovery sheet reads
        // as it did (§16.112 pass E3).
        let note = self
            .opened_sheet
            .as_ref()
            .and_then(|sheet| notes::Note::from_bytes(&sheet.note));
        if let Some(note) = note
            && !note.is_empty()
        {
            self.set_sheet_note(WalletRef::Policy(wallet), note);
        }
        self.sync_kept();
        self.open_wallet(wallet);
    }

    /// The name given to `wallet`, if it has one.
    pub(crate) fn wallet_name(&self, wallet: WalletRef) -> Option<&str> {
        match wallet {
            WalletRef::Key(_) => None,
            WalletRef::Policy(policy) => self.names.policy_name(self.wallets.get(policy)?),
            WalletRef::Typed => None,
        }
    }

    /// What a person calls `wallet`: the name it was given, or what it
    /// is — the fingerprint of a loaded key's own wallet, the shape of a
    /// policy. Home's rows, a wallet's title, the Forget table and the
    /// swap card all name a wallet this way.
    pub(crate) fn wallet_call_name(&self, wallet: WalletRef) -> String {
        if let Some(name) = self.wallet_name(wallet) {
            return String::from(name);
        }
        match wallet {
            WalletRef::Key(key) => match self.keys.get(key) {
                Some(k) => text::fingerprint_hex(k.fingerprint),
                None => String::from(self.strings().wallet_title),
            },
            // unreachable for a name, kept so one match draws every
            // reference.
            WalletRef::Policy(policy) => match self.wallets.get(policy) {
                Some(p) => self.wallet_label(p),
                None => String::from(self.strings().wallet_title),
            },
            WalletRef::Typed => String::from(self.strings().explore_typed),
        }
    }

    /// Whether `wallet` is called by its key's fingerprint — "73c5da0a ·
    /// SegWit", a single-key wallet with no name of its own — so that a
    /// title naming it is set in the mono face, as the key's own page is
    /// (`docs/DESIGN.md` §3). A name the person gave is words.
    pub(crate) fn wallet_called_by_key(&self, wallet: WalletRef) -> bool {
        if self.wallet_name(wallet).is_some() {
            return false;
        }
        match wallet {
            WalletRef::Key(key) => self.keys.get(key).is_some(),
            WalletRef::Policy(policy) => self.wallets.get(policy).is_some_and(|p| {
                p.keys()
                    .first()
                    .and_then(|k| k.fingerprint())
                    .is_some_and(|fp| self.wallet_label(p).starts_with(&text::fingerprint_hex(fp)))
            }),
            WalletRef::Typed => false,
        }
    }

    /// The same for a policy that is not in use yet, which has no index
    /// of its own: a name it was given under its checksum, or its shape.
    pub(crate) fn policy_call_name(&self, policy: &WalletPolicy) -> String {
        match self.names.policy_name(policy) {
            Some(name) => String::from(name),
            None => self.wallet_label(policy),
        }
    }

    /// Names `wallet`, or, given an empty name, takes its name away.
    /// The device keeps the names with the wallets, so the blob follows.
    fn set_wallet_name(&mut self, wallet: WalletRef, name: &str) {
        match wallet {
            WalletRef::Typed | WalletRef::Key(_) => {}
            WalletRef::Policy(policy) => {
                let Some(p) = self.wallets.get(policy) else {
                    return;
                };
                let checksum = p.checksum();
                self.names.set_checksum_name(&checksum, name);
            }
        }
        self.sync_kept();
    }

    /// Whether `policy` is one of the wallets in use.
    pub(crate) fn wallet_in_use(&self, policy: &WalletPolicy) -> bool {
        self.wallets.contains(policy)
    }

    /// Which alphabet the address keyboard shows: bech32 while what is
    /// typed can still become `bc1…`, `tb1…` or `bcrt1…`, base58 once
    /// it cannot. The case inside base58 is the person's, so it is left
    /// as they set it.
    fn address_layer(&mut self) {
        let mut mods = self.ui.modifiers();
        mods.shift = !osk_bip::address::is_bech32_prefix(self.verify.input(), self.network);
        self.ui.set_modifiers(mods);
    }

    /// Searches every loaded key and script type for `text` (UX.md G1).
    fn check_address(&mut self, text: &str) -> AddressResult {
        let address = match osk_bip::address::parse(text, self.network) {
            Ok(a) => a,
            Err(e) => return e.into(),
        };
        for (wallet, policy) in self.wallets.iter().enumerate() {
            if let Some((change, index)) =
                policy.find_address(self.network, &address, SEARCH_DEPTH - 1)
            {
                return AddressResult::Wallet {
                    wallet,
                    change,
                    index,
                };
            }
        }
        for i in 0..self.keys.len() {
            for script in ScriptType::ALL {
                let Some(account) = self.account(i, script) else {
                    continue;
                };
                if let Some((change, index)) = account.find_address(&address, SEARCH_DEPTH - 1) {
                    return AddressResult::Yours {
                        fingerprint: self.keys[i].fingerprint,
                        script,
                        change,
                        index,
                    };
                }
            }
        }
        AddressResult::NotFound
    }

    /// Drops the wizard, zeroizing everything it held.
    fn cancel_wizard(&mut self) {
        self.wizard = None;
        self.shares.zeroize();
        self.codex32.zeroize();
        self.sync_create();
        self.explore_entry = false;
        self.aezeed_entry = false;
        self.mask_deadline = None;
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The hold on a key wizard's confirm step: the session PIN step
    /// when the session has no PIN yet, otherwise the key is added.
    fn confirm_wizard(&mut self) {
        // The confirm step's button is dead while the shell owes the
        // session its entropy; nothing else reaches this.
        if self.session.entropy_pending() {
            return;
        }
        if self.session.has_pin() {
            self.finish_wizard();
            return;
        }
        match &mut self.wizard {
            Some(Wizard::Load(w)) if w.step() == Step::Confirm => w.go(Step::Pin),
            Some(Wizard::Create(w)) if w.step() == create::Step::Confirm => w.go(create::Step::Pin),
            _ => {}
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The PIN steps completed: the PIN becomes the session's and the key
    /// is added.
    fn pin_set(&mut self) {
        let pin = match &mut self.wizard {
            Some(Wizard::Load(w)) => w.take_pin(),
            Some(Wizard::Create(w)) => w.take_pin(),
            _ => None,
        };
        // The confirm step is dead until the shell has answered the
        // entropy request (§16.48), so the session key can seal the PIN
        // by the time this runs. If it could not, the step stays where
        // it is rather than adding a key behind a PIN nobody holds.
        if let Some(pin) = pin
            && self.session.set_pin(&pin)
        {
            self.finish_wizard();
        }
    }

    /// Adds the wizard's key, sealed under the session key, and goes
    /// Home.
    fn finish_wizard(&mut self) {
        let Some(w) = self.wizard.take() else {
            return;
        };
        self.mask_deadline = None;
        let created = matches!(w, Wizard::Create(_));
        let key = match w {
            Wizard::Load(w) => w.into_key(),
            Wizard::Create(w) => w.into_key(),
            // Add a wallet adds a wallet rather than a key, from its own
            // review's action.
            Wizard::Build(w) => {
                self.wizard = Some(Wizard::Build(w));
                return;
            }
            Wizard::Backup(b) => {
                self.wizard = Some(Wizard::Backup(b));
                return;
            }
        };
        let mut added = None;
        if let Some(mut key) = key
            && !self.keys.iter().any(|k| k.fingerprint == key.fingerprint)
        {
            key.seal(self.session.key_mut());
            self.keys.push(key);
            added = Some(self.keys.len() - 1);
        }
        self.after_key_added();
        // The first run (UX.md §7.1): a created key is proved by
        // checking one of its addresses, so the Result offers that; a
        // loaded key came from a backup that already exists.
        if !self.first_run_done
            && let Some(i) = added
        {
            if created {
                self.push(Screen::Created(i));
            } else {
                self.finish_first_run();
            }
        }
        // A device that keeps keys keeps this one too, with no word to
        // the element (§16.66). One that can keep keys and keeps none
        // yet asks now rather than through a row to find later
        // (§16.65): Back is "not now", and the key is added either way.
        if self.secret_kept {
            if added.is_some() {
                self.sync_kept();
            }
        } else if let Some(i) = added
            && self.keep_offered(i)
        {
            self.keep_offer = true;
            self.push(Screen::Keep(i));
        }
    }

    // ----- the encrypted backup (`osk_backup::oskb`) -----

    /// Bytes for one backup's salt and nonce, derived from the session
    /// key and a count of the backups made, so no two of a session draw
    /// the same salt.
    fn backup_seed(&mut self) -> [u8; osk_backup::oskb::SEED_LEN] {
        self.backups_made = self.backups_made.wrapping_add(1);
        let mut label = [0u8; 19];
        label[..15].copy_from_slice(b"osk-backup-seed");
        label[15..].copy_from_slice(&self.backups_made.to_le_bytes());
        let derived = self.session.key().derive(&label);
        let mut seed = [0u8; osk_backup::oskb::SEED_LEN];
        seed.copy_from_slice(&derived.expose()[..osk_backup::oskb::SEED_LEN]);
        seed
    }

    /// The name a saved backup is offered under. Every backup gets the
    /// same one: a file's name says nothing about the key it holds, and
    /// the shell's rule of never overwriting numbers a second file. The
    /// Result carries the key's fingerprint and the saved name, which is
    /// where a person learns which file is which.
    pub(crate) fn backup_file_name(&self) -> String {
        match self.form {
            Some((what @ FormFor::Backup(_), Form::Kdbx)) => self.kdbx_file_name(what),
            _ => String::from(self.strings().backup_file_name),
        }
    }

    /// The repeat matched: encrypt the key's words under it. The words
    /// are open only for this call, and both typings are wiped by
    /// [`BackupFlow::set_backup`].
    fn make_backup(&mut self) {
        let cost = self.export_cost();
        if self.form_is_kdbx() {
            let made = self.make_kdbx_backup(cost);
            self.backup_made(made);
            return;
        }
        let seed = self.backup_seed();
        let Some(Wizard::Backup(b)) = &self.wizard else {
            return;
        };
        // A key with words is sealed as its words; one without — a
        // SLIP-39 or a codex32 key — is sealed as its master seed, which
        // is the same key by another door (§16.112 rule 1).
        let made = match self.keys.get(b.key()).filter(|k| k.has_mnemonic()) {
            Some(_) => self
                .backup_words(b, |m| {
                    osk_backup::oskb::seal(m, b.pass_bytes(), cost, &seed)
                })
                .flatten(),
            None => self
                .keys
                .get(b.key())
                .and_then(|k| {
                    k.seed(self.session.key(), |bytes| {
                        osk_backup::oskb::seal_seed(bytes, b.pass_bytes(), cost, &seed)
                    })
                })
                .flatten(),
        };
        self.backup_made(made);
    }

    /// The same key as a KDBX 4 database: one entry titled by the
    /// fingerprint, the words or the master seed in the Password field,
    /// what they are in the Notes (§16.112 pass E2).
    fn make_kdbx_backup(&mut self, cost: osk_backup::Cost) -> Option<Vec<u8>> {
        let seed = self.kdbx_seed();
        let s = self.strings();
        let Some(Wizard::Backup(b)) = &self.wizard else {
            return None;
        };
        let key = self.keys.get(b.key())?;
        let title = text::fingerprint_hex(key.fingerprint);
        if key.has_mnemonic() {
            return self
                .backup_words(b, |m| {
                    let notes = strings::fill(
                        s.kdbx_words_notes,
                        &[
                            &alloc::format!("{}", m.word_count()),
                            text::language_name(m.language()),
                        ],
                    );
                    let mut words: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
                    for (i, &index) in m.indices().iter().enumerate() {
                        if i > 0 {
                            words.push(b' ');
                        }
                        words.extend_from_slice(m.language().word(index).as_bytes());
                    }
                    osk_backup::kdbx::write(
                        &[osk_backup::kdbx::Item {
                            title: title.as_bytes(),
                            notes: notes.as_bytes(),
                            secret: &words,
                        }],
                        b.pass_bytes(),
                        cost,
                        &seed,
                    )
                })
                .flatten();
        }
        key.seed(self.session.key(), |bytes| {
            let notes = strings::fill1(s.kdbx_seed_notes, &alloc::format!("{}", bytes.len()));
            let mut hex: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
            for &byte in bytes {
                hex.extend_from_slice(alloc::format!("{byte:02x}").as_bytes());
            }
            osk_backup::kdbx::write(
                &[osk_backup::kdbx::Item {
                    title: title.as_bytes(),
                    notes: notes.as_bytes(),
                    secret: &hex,
                }],
                b.pass_bytes(),
                cost,
                &seed,
            )
        })
        .flatten()
    }

    /// What became of a key's backup: the Result, or the flow closing
    /// where nothing was made.
    fn backup_made(&mut self, made: Option<Vec<u8>>) {
        match made {
            Some(bytes) => {
                if let Some(Wizard::Backup(b)) = &mut self.wizard {
                    b.set_backup(bytes);
                }
                self.mask_deadline = None;
                self.ui.set_scroll(ids::SCROLL, 0);
            }
            // Nothing was made, so there is nothing to show: the flow
            // closes the way it does for a key whose words are gone.
            None => self.cancel_wizard(),
        }
    }

    /// A tap on one of the gatherer's screens, which are Create a key's
    /// own: the shutter and the mixed result's reference rows belong to
    /// the app, the rest to the wizard.
    fn tap_gather(&mut self, id: Id, network: Network) {
        let now = self.now_ms;
        let class = self.class();
        let pane = self.words_pane_dp();
        let Some(Wizard::Backup(b)) = &mut self.wizard else {
            return;
        };
        let Some(g) = b.gather_mut() else {
            return;
        };
        if id == ids::CREATE_SHUTTER && g.step() == create::Step::Camera {
            self.create_shutter();
            self.sync_create();
            self.render();
            return;
        }
        if g.step() == create::Step::MixResult
            && let Some(i) = ids::index_in(id, ids::CREATE_MIX_BASE, osk_entropy::MIX_SOURCES.len())
            && let Some(c) = g.mix_commitments().get(i).copied()
        {
            let title = self.strings().create_mixed_title;
            self.open_compare(title, text::hex(&c));
            return;
        }
        let per_page = views::words::per_page(class, pane, g.language());
        g.tap(id, network, now, per_page);
        // The gatherer's words are the part: it never shows them.
        let ready = g.step() == create::Step::Words;
        self.sync_create();
        if ready {
            self.part_gathered(network);
            return;
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// "Random parts from?" confirmed, and after each random part: the
    /// chosen source's own steps for part `at` of the split, at the
    /// key's word count and wordlist.
    fn start_gather(&mut self, at: usize) {
        let Some(Wizard::Backup(b)) = &self.wizard else {
            return;
        };
        let key = b.key();
        let session = &self.session;
        let words = self
            .keys
            .get(key)
            .and_then(|k| k.mnemonic(session.key(), |m| (m.word_count() as u8, m.language())));
        let Some((count, lang)) = words else {
            self.cancel_wizard();
            return;
        };
        let has_camera = self.has_camera;
        let tier_d = self.tier == AssuranceTier::D;
        if let Some(Wizard::Backup(b)) = &mut self.wizard {
            b.start_gather(at, count, lang, has_camera, tier_d);
        }
        self.sync_create();
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The gatherer has reached its words, which are one random part:
    /// the part is kept and the next one starts, or the split is made.
    fn part_gathered(&mut self, network: Network) {
        let Some(Wizard::Backup(b)) = &mut self.wizard else {
            return;
        };
        let count = b.part_count();
        let Some(at) = b.take_gathered() else {
            self.cancel_wizard();
            return;
        };
        if at + 2 < count {
            self.start_gather(at + 1);
        } else {
            self.make_split(network);
            self.sync_create();
        }
    }

    /// The chevron inside the gatherer: its own steps first, then the
    /// source choice for the first part, and the part before this one
    /// for any later part. A part left behind is forgotten either way.
    fn gather_back(&mut self) {
        let Some(Wizard::Backup(b)) = &mut self.wizard else {
            return;
        };
        let at = b.gather_at();
        let leaving = match b.gather_mut() {
            Some(g) => !g.back(),
            None => true,
        };
        // The first part has the source choice behind it, and leaving
        // it drops the choice's whole run; a later part has the part
        // before it, whose entry opens again empty.
        if leaving && at == 0 {
            b.leave_gather();
        } else if leaving {
            self.start_gather(at - 1);
        }
        self.sync_create();
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The last random part is in: the split itself
    /// (`docs/PLANNING.md` §8.2 item 14, §16.92).
    ///
    /// Every part but the last was made by the person, from the source
    /// they chose, in the flow's own gatherer; the last is the XOR of
    /// the key's entropy and those, so that XORing all of them gives
    /// the key back. Nothing is stored: the parts live in the flow and
    /// leaving it zeroizes them.
    fn make_split(&mut self, network: Network) {
        let Some(Wizard::Backup(b)) = &self.wizard else {
            return;
        };
        let (key, count) = (b.key(), b.part_count());
        let mut randoms = [[0u8; 32]; osk_entropy::MAX_XOR_PARTS];
        for (i, part) in randoms.iter_mut().enumerate().take(count.saturating_sub(1)) {
            let Some(gathered) = b.part(i) else {
                return;
            };
            part[..gathered.len()].copy_from_slice(gathered);
        }
        let session = &self.session;
        let made = self.keys.get(key).and_then(|k| {
            k.mnemonic(session.key(), |m| {
                let lang = m.language();
                let entropy = m.entropy();
                let len = entropy.expose().as_bytes().len();
                let mut xor = osk_entropy::SeedXor::new();
                let mut parts = [[0u8; 32]; osk_entropy::MAX_XOR_PARTS];
                for (i, part) in parts.iter_mut().enumerate().take(count - 1) {
                    part[..len].copy_from_slice(&randoms[i][..len]);
                    if xor.push(&part[..len]).is_err() {
                        return None;
                    }
                }
                if xor.push(entropy.expose().as_bytes()).is_err() {
                    return None;
                }
                let last = xor.entropy().ok()?;
                parts[count - 1][..len].copy_from_slice(last.as_bytes());
                let mut prints = [Fingerprint([0u8; 4]); osk_entropy::MAX_XOR_PARTS];
                for (i, print) in prints.iter_mut().enumerate().take(count) {
                    let part = Mnemonic::from_entropy(lang, &parts[i][..len]).ok()?;
                    let seed = part.to_seed(b"").ok()?;
                    *print = MasterKey::from_seed(&seed, network).fingerprint();
                }
                Some((parts, prints, len))
            })
        });
        randoms.zeroize();
        let Some(Some((mut parts, prints, len))) = made else {
            self.cancel_wizard();
            return;
        };
        if let Some(Wizard::Backup(b)) = &mut self.wizard {
            for i in 0..count {
                b.set_part(i, &parts[i][..len], prints[i]);
            }
            b.show_parts();
        }
        parts.zeroize();
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// "Print template" on the steel screen: a blank numbered table of
    /// the key's word count, with no words in it. It is a template, so
    /// it carries nothing of the key but how many rows it has.
    fn write_steel_template(&mut self) {
        let Some(Wizard::Backup(b)) = &self.wizard else {
            return;
        };
        let count = self
            .keys
            .get(b.key())
            .and_then(|k| k.mnemonic(self.session.key(), Mnemonic::word_count));
        let Some(count) = count else {
            return;
        };
        let s = self.strings();
        let mut text = String::from(s.steel_template_heading);
        text.push('\n');
        for i in 1..=count {
            text.push_str(&alloc::format!("{i:>2}. ____ \u{00b7} ____ \u{00b7}\n"));
        }
        self.commands.push_back(Command::WriteFile {
            kind: FileKind::Any,
            name_hint: String::from(s.steel_template_file),
            bytes: text.into_bytes(),
        });
    }

    /// The backup result's two actions.
    fn tap_encrypted(&mut self, id: Id) {
        let Some(Wizard::Backup(b)) = &self.wizard else {
            return;
        };
        if id == ids::BACKUP_SHOW_QR {
            if let Some(Wizard::Backup(b)) = &mut self.wizard {
                b.show_qr();
            }
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if b.save() == sign::Save::Waiting {
            return;
        }
        let name_hint = self.backup_file_name();
        let Some(Wizard::Backup(b)) = &mut self.wizard else {
            return;
        };
        let bytes = b.backup().to_vec();
        b.mark_saving();
        self.commands.push_back(Command::WriteFile {
            kind: FileKind::Any,
            name_hint,
            bytes,
        });
    }

    /// A scanned or read encrypted backup: the passphrase that opens it.
    fn backup_scanned(&mut self, bytes: Vec<u8>) {
        // Nothing but the scanner reads a backup, so there is nowhere
        // else to put one.
        if let Some(s) = self.scan.as_mut() {
            s.set_backup(bytes);
            self.ui.set_scroll(ids::SCROLL, 0);
        }
    }

    /// ✓ on the scanner's passphrase entry: the Argon2id and the open.
    /// A tag that does not verify keeps the entry and says so.
    fn open_scanned_backup(&mut self) {
        let Some(s) = self.scan.as_ref() else {
            return;
        };
        if !s.backup_pass_ready() {
            return;
        }
        let network = self.network;
        match osk_backup::oskb::open_payload(s.backup(), s.backup_pass()) {
            Ok(osk_backup::oskb::Opened::Words(m)) => {
                self.load_scanned(&m);
                self.mask_deadline = None;
            }
            // A master seed is a key read from its secret, the way a
            // SLIP-39 or a codex32 key is: the wizard opens at its
            // confirmation with the key already built.
            Ok(osk_backup::oskb::Opened::Seed(seed)) => match LoadWizard::from_seed(&seed, network)
            {
                Some(wizard) => {
                    self.words_to_wizard(wizard);
                    self.mask_deadline = None;
                }
                None => {
                    let reason = String::from(self.strings().scan_reason_backup);
                    self.scan_failed(reason);
                }
            },
            Ok(osk_backup::oskb::Opened::Note(text)) => {
                self.mask_deadline = None;
                self.note_opened(&text);
            }
            Ok(osk_backup::oskb::Opened::Sheet(sheet)) => {
                self.mask_deadline = None;
                self.sheet_opened(&sheet);
            }
            Err(osk_backup::oskb::Error::Passphrase) => {
                if let Some(s) = self.scan.as_mut() {
                    s.backup_wrong_pass();
                }
            }
            Err(osk_backup::oskb::Error::Memory(mib)) => {
                let reason =
                    strings::fill1(self.strings().backup_needs_memory, &alloc::format!("{mib}"));
                self.scan_failed(reason);
            }
            Err(osk_backup::oskb::Error::Format) => {
                let reason = String::from(self.strings().scan_reason_backup);
                self.scan_failed(reason);
            }
        }
    }

    /// The Backup flow closed: record a passed quiz, return to Key detail.
    fn finish_backup(&mut self) {
        let mut verified = false;
        if let Some(Wizard::Backup(b)) = &self.wizard
            && b.step() == BackupStep::QuizResult
            && b.passed()
        {
            let i = b.key();
            {
                {
                    if let Some(k) = self.keys.get_mut(i) {
                        k.backup_verified = true;
                        verified = true;
                    }
                }
            }
        }
        self.cancel_wizard();
        // A passed quiz is part of what the device keeps about a key.
        if verified {
            self.sync_kept();
        }
    }

    fn forget(&mut self, key: usize) {
        let kept = self.keep_is_kept(key);
        if key < self.keys.len() {
            let removed = self.keys.remove(key);
            self.cache.retain(|c| c.fingerprint != removed.fingerprint);
            self.explore.key_removed(key);
        }
        // Forgetting a key this device keeps forgets the copy on the
        // device with it (§16.65, §16.66): a key that came back on the
        // next start, behind the pad this forget leaves showing, would
        // not have been forgotten. Forgetting the last one forgets the
        // blob.
        if kept {
            self.sync_kept();
        }
        self.go_home();
    }

    // ----- session: lock, wipe, exit, entropy -----

    /// Zeroizes every secret in memory: keys, caches, any wizard, the
    /// PSBT, the session PIN. The session key is rotated when the shell
    /// answers the entropy request this sends. The screen is left alone
    /// (UX.md C8: Settings shows that it happened); the timers and the
    /// lock state are cleared since there is nothing left to guard.
    ///
    /// The key kept on the device is not memory and stays. Only the
    /// wipes a person asks for remove it, through
    /// [`wipe_everything`](Self::wipe_everything).
    fn wipe_secrets(&mut self) {
        self.musig_session = None;
        self.drop_keys();
        self.keep.clear();
        self.keep_attempts = None;
        self.session.reset();
        self.request_entropy();
    }

    /// Drops every loaded key and everything derived from or in the
    /// middle of using one, and nothing else: the session, its PIN and
    /// the stored key's exchange stay as they are.
    fn drop_keys(&mut self) {
        self.cancel_wizard();
        self.open_finish = None;
        self.child = None;
        self.sign.clear();
        self.message = None;
        self.check = None;
        self.leave_scan();
        self.keys.clear();
        self.cache.clear();
        self.wallet_addresses = None;
        // A wallet is only in use while the keys that review it are, so
        // a wipe forgets the wallets with them (UX.md C8, I3).
        self.wallets.clear();
        // A name belongs to the wallet it names, and every wallet is
        // gone; so does the answer to "Keep this wallet?".
        self.names.clear();
        self.wallets_not_kept.clear();
        // A kept note and a kept sheet are in the blob, which the keys
        // are the protection of, so they go where the wallets go.
        self.notes_kept.clear();
        self.sheets_kept.clear();
        // The blob's key and the copy of its bytes belong to the keys
        // they were opened with.
        self.kept_blob = Vec::new();
        self.kept_kek = None;
    }

    /// The keys on the device follow the keys in memory (§16.66): a key
    /// added, forgotten or backed up while keys are kept rewrites the
    /// blob's keys record under the key that opened it, with no word to
    /// the element, and forgetting the last one forgets the blob.
    fn sync_kept(&mut self) {
        if !self.secret_kept {
            return;
        }
        let keys = self.kept_keys();
        if keys.is_empty() {
            self.secret_kept = false;
            self.kept_blob = Vec::new();
            self.kept_kek = None;
            self.keep_attempts = None;
            self.keep.clear();
            self.commands.push_back(Command::ForgetSecret);
            return;
        }
        self.refresh_kept_sheets();
        let seed = self.keep_seed();
        let mut blob = core::mem::take(&mut self.kept_blob);
        let wallets = self.kept_wallets_body();
        let notes = self.kept_notes();
        let ok = self.kept_kek.as_ref().is_some_and(|k| {
            k.with(self.session.key(), |kek| {
                osk_keep::rewrite(&mut blob, kek, &keys, &wallets, &notes, &seed)
            })
            .unwrap_or(false)
        });
        self.kept_blob = blob;
        // The key is held from the moment the blob is written or opened
        // until the keys go, and keys are in memory only between those
        // two points, so this is always the case; if it were not, the
        // device would keep what it had.
        if ok {
            let blob = self.kept_blob.clone();
            self.commands.push_back(Command::StoreSecret { blob });
        }
    }

    /// The wallets record's body: the wallets that are kept, and the
    /// names they have. A wallet left out of the record is simply not in
    /// the body, and is still the session's (`docs/PLANNING.md` §16.104
    /// rule 7).
    fn kept_wallets_body(&self) -> Vec<u8> {
        let wallets: Vec<WalletPolicy> = self
            .wallets
            .iter()
            .enumerate()
            .filter(|(i, _)| self.wallet_is_kept(*i))
            .map(|(_, p)| p.clone())
            .collect();
        osk_keep::wallets::body(&wallets, &self.names)
    }

    /// The notes record's items: every kept note as the container's
    /// kind-3 payload and every kept sheet as its kind-4 payload, which
    /// is what an `osk-backup` file of the same item carries
    /// (`docs/PLANNING.md` §16.112 pass E3).
    fn kept_notes(&self) -> osk_keep::notes::KeptNotes {
        let mut out = osk_keep::notes::KeptNotes::new();
        for note in &self.notes_kept {
            if let Some(item) =
                osk_keep::notes::KeptNote::new(osk_backup::oskb::KIND_NOTE, note.bytes())
            {
                out.push(item);
            }
        }
        for sheet in &self.sheets_kept {
            let Some(payload) =
                osk_backup::oskb::sheet_payload(&sheet.descriptor, &sheet.name, &sheet.note)
            else {
                continue;
            };
            if let Some(item) =
                osk_keep::notes::KeptNote::new(osk_backup::oskb::KIND_SHEET, &payload)
            {
                out.push(item);
            }
        }
        out
    }

    /// How many slots of the record are taken.
    pub(crate) fn kept_items(&self) -> usize {
        self.notes_kept.len() + self.sheets_kept.len()
    }

    /// Whether the record has room for one more.
    pub(crate) fn kept_items_full(&self) -> bool {
        self.kept_items() >= osk_keep::notes::MAX_NOTES
    }

    /// A kept sheet follows the wallet it belongs to: a name given or a
    /// note written after the sheet was kept is in the record at the
    /// next write. A sheet whose wallet is not here is left as it was,
    /// which is what makes it still a document.
    fn refresh_kept_sheets(&mut self) {
        let mut fresh: Vec<(usize, String, String)> = Vec::new();
        for (i, sheet) in self.sheets_kept.iter().enumerate() {
            let Ok(descriptor) = core::str::from_utf8(&sheet.descriptor) else {
                continue;
            };
            let Some(wallet) = self
                .wallets
                .iter()
                .position(|w| w.to_descriptor_checksummed() == descriptor)
            else {
                continue;
            };
            let wallet = WalletRef::Policy(wallet);
            let name = self
                .wallet_name(wallet)
                .map(String::from)
                .unwrap_or_default();
            fresh.push((i, name, self.sheet_note_text(wallet)));
        }
        for (i, name, note) in fresh {
            let Some(sheet) = self.sheets_kept.get_mut(i) else {
                continue;
            };
            sheet.name = Zeroizing::new(name.into_bytes());
            sheet.note = Zeroizing::new(note.into_bytes());
        }
    }

    /// The items the record held, as the session holds them, and the
    /// note of every kept sheet whose wallet came back with it.
    fn load_kept_notes(&mut self, notes: osk_keep::notes::KeptNotes) {
        self.notes_kept.clear();
        self.sheets_kept.clear();
        for item in notes.iter() {
            match osk_backup::oskb::payload_of(item.kind(), item.payload()) {
                Some(osk_backup::oskb::Opened::Note(bytes)) => {
                    if let Some(note) = notes::Note::from_bytes(&bytes) {
                        self.notes_kept.push(note);
                    }
                }
                Some(osk_backup::oskb::Opened::Sheet(sheet)) => self.sheets_kept.push(sheet),
                _ => {}
            }
        }
        let mut restored: Vec<(usize, Vec<u8>)> = Vec::new();
        for sheet in &self.sheets_kept {
            let Ok(descriptor) = core::str::from_utf8(&sheet.descriptor) else {
                continue;
            };
            if let Some(i) = self
                .wallets
                .iter()
                .position(|w| w.to_descriptor_checksummed() == descriptor)
                && !sheet.note.is_empty()
            {
                restored.push((i, sheet.note.to_vec()));
            }
        }
        for (wallet, note) in restored {
            if let Some(note) = notes::Note::from_bytes(&note) {
                self.set_sheet_note(WalletRef::Policy(wallet), note);
            }
        }
    }

    /// Whether the note in hand is one of the kept ones.
    pub(crate) fn note_is_kept(&self) -> bool {
        !self.note.is_empty()
            && self
                .notes_kept
                .iter()
                .any(|n| n.bytes() == self.note.bytes())
    }

    /// Puts the note in hand in the record or takes it out, and
    /// rewrites it.
    fn set_note_kept(&mut self, kept: bool) {
        let bytes = self.note.bytes().to_vec();
        self.notes_kept.retain(|n| n.bytes() != bytes);
        if kept && !bytes.is_empty() && !self.kept_items_full() {
            let Some(note) = notes::Note::from_bytes(&bytes) else {
                return;
            };
            self.notes_kept.push(note);
        }
        self.sync_kept();
    }

    /// Which kept sheet is `wallet`'s, by the descriptor it states.
    fn kept_sheet_of(&self, wallet: WalletRef) -> Option<usize> {
        let descriptor = self.sheet_descriptor(wallet)?;
        self.sheets_kept
            .iter()
            .position(|s| s.descriptor.as_slice() == descriptor.as_bytes())
    }

    /// Whether `wallet`'s recovery sheet is on the device.
    pub(crate) fn sheet_is_kept(&self, wallet: WalletRef) -> bool {
        self.kept_sheet_of(wallet).is_some()
    }

    /// Puts `wallet`'s sheet in the record or takes it out.
    fn set_sheet_kept(&mut self, wallet: WalletRef, kept: bool) {
        match (self.kept_sheet_of(wallet), kept) {
            (Some(i), false) => {
                self.sheets_kept.remove(i);
            }
            (None, true) => {
                let Some(descriptor) = self.sheet_descriptor(wallet) else {
                    return;
                };
                if self.kept_items_full() {
                    return;
                }
                let name = self
                    .wallet_name(wallet)
                    .map(String::from)
                    .unwrap_or_default();
                let note = self.sheet_note_text(wallet);
                self.sheets_kept.push(osk_backup::oskb::Sheet {
                    descriptor: Zeroizing::new(descriptor.into_bytes()),
                    name: Zeroizing::new(name.into_bytes()),
                    note: Zeroizing::new(note.into_bytes()),
                });
            }
            _ => return,
        }
        self.sync_kept();
    }

    /// The kept sheets no wallet in use accounts for. Each is listed
    /// under Tools › Notes until its wallet is added again.
    pub(crate) fn orphan_sheets(&self) -> Vec<usize> {
        (0..self.sheets_kept.len())
            .filter(|i| {
                let sheet = &self.sheets_kept[*i];
                match core::str::from_utf8(&sheet.descriptor) {
                    Ok(d) => !self
                        .wallets
                        .iter()
                        .any(|w| w.to_descriptor_checksummed() == d),
                    Err(_) => true,
                }
            })
            .collect()
    }

    /// What a kept sheet with no wallet is called on that list: the
    /// name the person gave the wallet, or the descriptor itself.
    pub(crate) fn orphan_sheet_label(&self, i: usize) -> String {
        let Some(sheet) = self.sheets_kept.get(i) else {
            return String::new();
        };
        let name = String::from_utf8_lossy(&sheet.name).into_owned();
        if name.is_empty() {
            String::from_utf8_lossy(&sheet.descriptor).into_owned()
        } else {
            name
        }
    }

    /// The notes kept on the device, by the first line each one
    /// carries, which is what names a note anywhere it is listed.
    pub(crate) fn kept_note_label(&self, i: usize) -> String {
        self.notes_kept
            .get(i)
            .map_or_else(String::new, |n| String::from(n.text()))
    }

    /// How many notes are kept.
    pub(crate) fn kept_note_count(&self) -> usize {
        self.notes_kept.len()
    }

    /// Whether the note at `i` is the one in hand, which the list shows
    /// once.
    pub(crate) fn kept_note_in_hand(&self, i: usize) -> bool {
        self.notes_kept
            .get(i)
            .is_some_and(|n| n.bytes() == self.note.bytes())
    }

    /// Every loaded key the device keeps, once each, in the order Keys
    /// lists them: the words of a key typed or created, or the master
    /// secret of a SLIP-39 key (§16.107 rule 6).
    fn kept_keys(&self) -> osk_keep::KeptKeys {
        let mut keys = osk_keep::KeptKeys::new();
        let mut seen: Vec<Secret<<MnemonicBytes as SealedBytes>::Bytes>> = Vec::new();
        for k in &self.keys {
            // A derived key is the session's, not the device's (§16.67).
            if k.derived {
                continue;
            }
            let secret = match k.mnemonic(self.session.key(), load::mnemonic_bytes) {
                Some(mnemonic) => {
                    let mut bytes = <MnemonicBytes as SealedBytes>::ZEROED;
                    mnemonic.write_bytes(&mut bytes);
                    let bytes = Secret::new(bytes);
                    // Two keys over the same words — one with a
                    // passphrase — are one set of words on the device.
                    if seen.contains(&bytes) {
                        continue;
                    }
                    seen.push(bytes);
                    osk_keep::KeptSecret::Words(mnemonic)
                }
                // A SLIP-39 key read under a passphrase is never in the
                // blob: the passphrase is applied when the shares are
                // read, so the secret would carry it (§16.107 rule 3).
                None if k.has_passphrase => continue,
                // A seed the slot has no room for is not kept: a
                // 64-byte codex32 key is the session's (§16.109 rule 1).
                None if !k.is_keepable() => continue,
                None => {
                    let Some(Some(secret)) = k.seed(self.session.key(), SeedBytes::new) else {
                        continue;
                    };
                    osk_keep::KeptSecret::MasterSecret {
                        secret,
                        codex32: k.kind() == load::KeyKind::Codex32,
                    }
                }
            };
            keys.push(osk_keep::Kept {
                secret,
                backup_verified: k.backup_verified,
            });
        }
        keys
    }

    /// Wipes memory and removes the key kept on the device with it: the
    /// two wipes a person asks for, Settings › Wipe all keys and Wipe
    /// and exit. The shell forgets the blob and the hardware keys behind
    /// it, so no copy of the bytes can be tried again.
    fn wipe_everything(&mut self) {
        self.wipe_secrets();
        if self.secret_kept {
            self.secret_kept = false;
            self.commands.push_back(Command::ForgetSecret);
        }
    }

    /// Wipes memory and returns to the empty Home (auto-wipe, five wrong
    /// PINs, a lock on Tier D). The stored key stays: a timeout is not a
    /// decision to give up the key on the device.
    fn wipe(&mut self) {
        self.wipe_secrets();
        self.go_home();
    }

    /// Wipes, shows the terminal screen and asks the shell to exit
    /// (UX.md I3, `docs/PLANNING.md` §8.5 #6). Every exit goes through
    /// here. The screen is what a shell that takes its time closing
    /// keeps showing, and it offers nothing to touch.
    fn exit(&mut self) {
        self.wipe_everything();
        self.stack.clear();
        self.ended_by_leave = false;
        self.screen = Screen::Ended;
        self.entered();
        self.commands.push_back(Command::Exit);
    }

    /// Locks the session (UX.md I1): every master key and derived cache
    /// is dropped, any flow in progress is cancelled (its secrets with
    /// it), and the lock screen shows until the PIN is entered. The
    /// sealed seeds stay, re-sealed under a fresh session key once the
    /// shell answers the entropy request. Nothing happens without keys.
    /// On Tier D a lock is a wipe: keys are not kept past it.
    fn lock(&mut self) {
        if !self.has_secrets() || !self.session.has_pin() {
            return;
        }
        if self.tier == AssuranceTier::D {
            self.wipe();
            return;
        }
        self.musig_session = None;
        self.cancel_wizard();
        self.go_home();
        for k in &mut self.keys {
            k.lock();
        }
        self.cache.clear();
        self.session.lock();
        self.request_entropy();
    }

    /// The right PIN was entered: master keys are rebuilt from the sealed
    /// seeds.
    fn unlock(&mut self) {
        let OpenSigner { keys, session, .. } = self;
        keys.retain_mut(|k| k.unlock(session.key()));
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// Input on the lock screen's PIN pad.
    fn lock_input(&mut self, input: KeyInput) {
        match input {
            KeyInput::Char(c) => {
                self.session.entry_push(c);
            }
            KeyInput::Backspace => self.session.entry_pop(),
            KeyInput::Done if self.kept_rows_offered() => self.lock_submit_kept(),
            KeyInput::Done => match self.session.submit() {
                Unlock::Unlocked => self.unlock(),
                Unlock::Wrong(_) => {}
                Unlock::Wiped => self.wipe(),
            },
            KeyInput::Shift | KeyInput::Symbols => {}
        }
    }

    /// The lock screen on a device that keeps a key is the stored key's
    /// pad as well (§16.63): the session PIN opens the session, and any
    /// other PIN is tried against the blob, so the duress PIN works here
    /// and a wrong PIN counts toward the stored key's removal rather
    /// than toward a wipe of memory.
    fn lock_submit_kept(&mut self) {
        if self.keep.step.is_some() || !self.session.entry().is_complete() {
            return;
        }
        // The digits go to the stored key's pad before the check clears
        // them.
        self.keep.set_entry(self.session.entry());
        if self.session.try_unlock() {
            // The right PIN ends the run of wrong ones counted against
            // the blob on this screen.
            if self.keep_attempts.take().is_some() && osk_keep::reset_attempts(&mut self.keep.blob)
            {
                let blob = self.keep.blob.clone();
                self.commands.push_back(Command::StoreSecret { blob });
            }
            self.keep.clear();
            self.unlock();
            return;
        }
        // Opening the blob loads a key, and so waits for the shell's
        // entropy as the stored key's own pad does.
        if self.session.entropy_pending() {
            self.keep.clear_entry();
            return;
        }
        self.keep.step = Some(keep::Step::Opening);
        self.commands.push_back(Command::LoadSecret);
    }

    /// Asks the shell for entropy, unless a request is outstanding.
    fn request_entropy(&mut self) {
        if self.session.request_entropy() {
            self.commands.push_back(Command::RequestEntropy);
        }
    }

    /// The shell answered: the session key rotates and every sealed
    /// value (or plaintext one, under a weak key until now) is sealed
    /// under the new key. The old key zeroizes when this returns.
    /// `false` for an answer nobody asked for.
    ///
    /// Re-sealing is also where each loaded key's curve context takes
    /// the new blind, so fresh entropy reaches the signing side as well
    /// as the sealing side (security review M1).
    fn entropy(&mut self, bytes: &[u8; 32]) -> bool {
        let Some(old) = self.session.entropy(bytes) else {
            return false;
        };
        let OpenSigner {
            keys,
            session,
            kept_kek,
            ..
        } = self;
        for k in keys.iter_mut() {
            k.reseal(&old, session.key_mut());
        }
        if let Some(k) = kept_kek
            && let Ok(sealed) = k.reseal(&old, session.key_mut())
        {
            *k = sealed;
        }
        session.reseal(&old);
        true
    }

    /// Runs the timers at `now_ms`. Returns whether the screen changed
    /// (a lock or a wipe fired).
    fn timers(&mut self, now_ms: u64) -> bool {
        if !self.has_secrets() {
            return false;
        }
        if self.session.wipe_deadline().is_some_and(|d| now_ms >= d) {
            self.wipe();
            return true;
        }
        if !self.session.is_locked() && now_ms >= self.session.lock_deadline() {
            self.lock();
            return true;
        }
        false
    }

    fn set_network(&mut self, network: Network) {
        if network == self.network {
            return;
        }
        self.network = network;
        for k in &mut self.keys {
            if let Some(m) = &k.master {
                // The relabelled key carries the blinded context of the
                // one it was made from (security review M1).
                k.master = Some(m.for_network(network));
            }
            k.network = network;
        }
        self.cache.clear();
        self.explore.rebuild(network);
        self.explore_cache = None;
    }

    // ----- a key kept on the device (`keep`, PLANNING §15 item 32) -----

    /// What backs the shell's secure element, which About states.
    pub fn secure(&self) -> SecureHardware {
        self.secure
    }

    /// What the platform said about this device's boot, which About
    /// states.
    pub fn boot(&self) -> BootState {
        self.boot
    }

    /// Whether the app refuses to run: the platform says the running
    /// system is not the one it verified, which is an unlocked
    /// bootloader or a verification that failed. Nothing under the app —
    /// its memory, its screen, its files, the element it asks for a key
    /// — is then what the tiers describe, so the only screen is the
    /// refusal (`docs/PLANNING.md` §16, the 2026-09-11 review §5). A
    /// shell that cannot tell reports [`BootState::Unknown`] and runs.
    pub(crate) fn boot_refused(&self) -> bool {
        self.boot == BootState::Unverified
    }

    /// Whether the shell says a blob is on the device.
    pub fn secret_kept(&self) -> bool {
        self.secret_kept
    }

    /// Whether a key can be kept here at all: Tier B is where a device
    /// keeps anything (§3), and a secure element is what makes the
    /// stored bytes worth writing. The core trusts what the shell
    /// reported in [`DisplayInfo`].
    fn can_keep(&self) -> bool {
        self.tier == AssuranceTier::B && self.secure != SecureHardware::None
    }

    /// Whether the key menu of `key` offers "Keep on this device": the
    /// words are what is stored, and the session PIN is what encrypts
    /// them, so a key without either cannot be kept.
    pub(crate) fn keep_offered(&self, key: usize) -> bool {
        self.can_keep()
            && !self.secret_kept
            && self.session.has_pin()
            && self
                .keys
                .get(key)
                .is_some_and(|k| k.is_keepable() && !k.derived)
    }

    /// Whether `key` is on the device: every loaded key the device can
    /// keep is,
    /// once the device keeps anything (§16.66), except a key derived
    /// from another, which is the session's alone (§16.67).
    pub(crate) fn keep_is_kept(&self, key: usize) -> bool {
        self.secret_kept
            && self
                .keys
                .get(key)
                .is_some_and(|k| k.is_keepable() && !k.derived)
    }

    /// Whether a wallet's page carries "Kept on this device": there is a
    /// blob for a wallet to be in, which is Tier B keeping keys.
    pub(crate) fn wallet_keep_offered(&self) -> bool {
        self.kept_rows_offered()
    }

    /// Whether `wallet` is in the blob's wallets record. A wallet is
    /// kept unless it was left out: the choice at its creation, or its
    /// own row afterwards (§16.104 rule 7).
    pub(crate) fn wallet_is_kept(&self, wallet: usize) -> bool {
        self.wallets
            .get(wallet)
            .is_some_and(|p| !self.wallets_not_kept.contains(&p.checksum()))
    }

    /// Puts `wallet` in the record or takes it out, and rewrites it.
    fn set_wallet_kept(&mut self, wallet: usize, kept: bool) {
        let Some(checksum) = self.wallets.get(wallet).map(WalletPolicy::checksum) else {
            return;
        };
        self.wallets_not_kept.retain(|c| *c != checksum);
        if !kept {
            self.wallets_not_kept.push(checksum);
        }
        self.sync_kept();
    }

    /// Whether adding `wallet` asks whether to keep it: a device that
    /// keeps keys, and a wallet this device built over a passphrase key
    /// it holds. The kept descriptor is evidence the wallet exists, so
    /// the answer is the person's and it is "No" until they say
    /// otherwise (§16.104 rule 7).
    fn asks_to_keep_wallet(&self, wallet: usize) -> bool {
        self.kept_rows_offered()
            && self.wallet_keys(wallet).iter().any(|(_, loaded)| {
                loaded.is_some_and(|k| self.keys.get(k).is_some_and(|k| k.has_passphrase))
            })
    }

    /// Whether the stored key's pad is the screen: a device that keeps a
    /// key opens on its PIN pad and returns to it whenever no key is
    /// loaded, so the PIN is the front door to everything (§16.64). The
    /// lock screen, which needs a loaded key, and this never show at
    /// once.
    pub(crate) fn front_door(&self) -> bool {
        self.kept_rows_offered()
            && !self.has_secrets()
            && !self.selftest_failed()
            && !self.boot_refused()
            && self.screen != Screen::Ended
    }

    /// Whether a key is kept on this device: Settings offers the duress
    /// PIN, and the PIN pads try the blob.
    pub(crate) fn kept_rows_offered(&self) -> bool {
        self.can_keep() && self.secret_kept
    }

    /// Attempts left on the stored key, once a wrong PIN has said so.
    pub(crate) fn kept_attempts_left(&self) -> Option<u8> {
        self.keep_attempts
    }

    /// Bytes for the salts and nonces of one write, derived from the
    /// session key and a count of the writes, so no two writes of a
    /// session draw the same.
    fn keep_seed(&mut self) -> [u8; 32] {
        self.keep_writes = self.keep_writes.wrapping_add(1);
        let mut label = [0u8; 17];
        label[..13].copy_from_slice(b"osk-keep-seed");
        label[13..].copy_from_slice(&self.keep_writes.to_le_bytes());
        let derived = self.session.key().derive(&label);
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&derived.expose()[..32]);
        seed
    }

    /// The Hold was held: a fresh header, the session PIN stretched
    /// against it, and the tag for that PIN's challenge.
    fn start_keeping(&mut self, key: usize) {
        if !self.keep_offered(key) {
            return;
        }
        let seed = self.keep_seed();
        let header = osk_keep::new_header(self.keep_cost(), &seed);
        let guess = self
            .session
            .with_pin(|pin| osk_keep::challenge(&header, pin))
            .flatten();
        let Some(guess) = guess else {
            return;
        };
        let salt = *guess.challenge();
        self.keep.set_seed(seed);
        self.keep.header = Some(header);
        self.keep.guess = Some(guess);
        self.keep.key = key;
        self.keep.step = Some(keep::Step::Keeping);
        self.commands.push_back(Command::SecureMac { salt });
    }

    /// The stored key's pad. The PIN is not checked here: the blob and
    /// the secure element are, in that order.
    fn stored_key_input(&mut self, input: KeyInput) {
        match input {
            KeyInput::Char(c) => self.keep.push(c),
            KeyInput::Backspace => self.keep.pop(),
            KeyInput::Done => {
                // Opening the blob loads a key and makes its PIN the
                // session PIN, so it waits for the shell's entropy for
                // the same reason "Add key" does (§16.48, §16.51).
                if self.keep.pin_complete()
                    && self.keep.step.is_none()
                    && !self.session.entropy_pending()
                {
                    self.keep.step = Some(keep::Step::Opening);
                    self.commands.push_back(Command::LoadSecret);
                }
            }
            KeyInput::Shift | KeyInput::Symbols => {}
        }
    }

    /// The duress pad: the PIN once, then again, then the rewrite.
    fn duress_input(&mut self, input: KeyInput) {
        match input {
            KeyInput::Char(c) => self.keep.push(c),
            KeyInput::Backspace => self.keep.pop(),
            KeyInput::Done => {
                if self.keep.step.is_some() {
                    return;
                }
                if !self.keep.repeat {
                    self.keep.repeat_duress();
                } else if self.keep.confirm_duress() {
                    self.keep.step = Some(keep::Step::Duress);
                    self.commands.push_back(Command::LoadSecret);
                }
            }
            KeyInput::Shift | KeyInput::Symbols => {}
        }
    }

    /// The shell answered [`Command::LoadSecret`]. A blob this build
    /// cannot read is no blob at all.
    fn secret_blob(&mut self, blob: Option<Vec<u8>>) {
        let step = self.keep.step;
        if !matches!(step, Some(keep::Step::Opening | keep::Step::Duress)) {
            return;
        }
        let Some(blob) = blob.filter(|b| osk_keep::header(b).is_some()) else {
            // Nothing came back, or nothing this build reads: the shell
            // is not keeping a key this app can open, whatever it said
            // at start, and the pad that asks for its PIN goes.
            self.secret_kept = false;
            self.kept_blob = Vec::new();
            self.kept_kek = None;
            self.keep_attempts = None;
            self.keep_failed();
            return;
        };
        let header = osk_keep::header(&blob).expect("checked above");
        // The element authenticates a challenge derived from the PIN on
        // the pad, so its answer is worth nothing for any other PIN.
        let Some(guess) = self.keep.with_pin(|pin| osk_keep::challenge(&header, pin)) else {
            self.keep_failed();
            return;
        };
        let salt = *guess.challenge();
        self.keep.blob = blob;
        self.keep.header = Some(header);
        self.keep.guess = Some(guess);
        self.commands.push_back(Command::SecureMac { salt });
    }

    /// The secure element answered. This is where every record is
    /// opened or written: without the tag nothing here is possible.
    fn secure_mac(&mut self, mac: &[u8; 32]) {
        match self.keep.step {
            Some(keep::Step::Keeping) => self.write_blob(mac),
            Some(keep::Step::Opening) => self.open_blob(mac),
            Some(keep::Step::Duress) => self.write_duress(mac),
            _ => {}
        }
    }

    /// Writes a fresh blob for the key on the Hold.
    fn write_blob(&mut self, mac: &[u8; 32]) {
        let guess = self.keep.guess.take();
        let (Some(header), Some(guess)) = (self.keep.header, guess) else {
            self.keep_failed();
            return;
        };
        let seed = *self.keep.seed();
        let keys = self.kept_keys();
        if keys.is_empty() {
            self.keep_failed();
            return;
        }
        let wallets = self.kept_wallets_body();
        self.refresh_kept_sheets();
        let notes = self.kept_notes();
        let (blob, kek) = osk_keep::write(&header, mac, &guess, &keys, &wallets, &notes, &seed);
        self.kept_kek = osk_crypto::Sealed::seal(self.session.key_mut(), kek).ok();
        self.kept_blob = blob.clone();
        self.keep.step = Some(keep::Step::Storing(keep::Landing::KeyMenu));
        self.commands.push_back(Command::StoreSecret { blob });
    }

    /// Tries the typed PIN against both records.
    fn open_blob(&mut self, mac: &[u8; 32]) {
        let Some(guess) = self.keep.guess.take() else {
            self.keep_failed();
            return;
        };
        let opened = osk_keep::open(&self.keep.blob, mac, &guess);
        match opened {
            osk_keep::Opened::Keys(keys, wallets, notes, kek) => {
                // The right PIN ends the run of wrong ones (§16.63).
                if osk_keep::reset_attempts(&mut self.keep.blob) {
                    let blob = self.keep.blob.clone();
                    self.commands.push_back(Command::StoreSecret { blob });
                }
                // On the lock screen this is the stored keys' PIN typed
                // over a session whose keys were loaded under another:
                // the session it opens is the stored keys'.
                if self.session.is_locked() {
                    self.drop_keys();
                    self.session.reset();
                }
                if self.load_kept(keys) {
                    // The wallets came back with the keys they were kept
                    // beside (§16.72).
                    let (wallets, names) = osk_keep::wallets::wallets(&wallets);
                    self.wallets = wallets;
                    self.names = names;
                    // The notes and sheets came back beside them
                    // (§16.112 pass E3).
                    self.load_kept_notes(notes);
                    self.kept_blob = core::mem::take(&mut self.keep.blob);
                    self.kept_kek = osk_crypto::Sealed::seal(self.session.key_mut(), kek).ok();
                    self.keep.step = None;
                    self.keep.clear();
                    self.keep_attempts = None;
                    self.go_home();
                } else {
                    self.keep_failed();
                }
            }
            // The duress PIN: everything goes and the app comes back
            // with no stored key and nothing said about one.
            osk_keep::Opened::Duress => {
                self.keep.step = Some(keep::Step::Forgetting(keep::Landing::Empty));
                self.commands.push_back(Command::ForgetSecret);
            }
            osk_keep::Opened::Wrong => {
                let left = osk_keep::count_attempt(&mut self.keep.blob);
                if left == 0 {
                    self.keep.step = Some(keep::Step::Forgetting(keep::Landing::Removed));
                    self.commands.push_back(Command::ForgetSecret);
                    return;
                }
                self.keep_attempts = Some(left);
                self.keep.clear_entry();
                let blob = self.keep.blob.clone();
                self.keep.step = Some(keep::Step::Storing(keep::Landing::Stay));
                self.commands.push_back(Command::StoreSecret { blob });
            }
        }
    }

    /// Rewrites the duress record of the blob the shell handed back.
    fn write_duress(&mut self, mac: &[u8; 32]) {
        let seed = self.keep_seed();
        let Some(guess) = self.keep.guess.take() else {
            self.keep_failed();
            return;
        };
        let mut blob = core::mem::take(&mut self.keep.blob);
        let ok = osk_keep::set_duress(&mut blob, mac, Some(&guess), &seed);
        self.keep.blob = blob;
        if !ok {
            self.keep_failed();
            return;
        }
        let blob = self.keep.blob.clone();
        self.kept_blob = blob.clone();
        self.keep.step = Some(keep::Step::Storing(keep::Landing::Settings));
        self.commands.push_back(Command::StoreSecret { blob });
    }

    /// The shell answered [`Command::StoreSecret`].
    fn secret_stored(&mut self, ok: bool) {
        let Some(keep::Step::Storing(landing)) = self.keep.step else {
            return;
        };
        self.keep.step = None;
        if !ok {
            self.keep_failed();
            return;
        }
        self.secret_kept = true;
        self.land(landing);
    }

    /// The shell answered [`Command::ForgetSecret`]: nothing is kept
    /// and nothing can be tried again.
    fn secret_forgotten(&mut self) {
        self.secret_kept = false;
        self.kept_blob = Vec::new();
        self.kept_kek = None;
        self.keep_attempts = None;
        let Some(keep::Step::Forgetting(landing)) = self.keep.step else {
            self.keep.clear();
            return;
        };
        self.keep.step = None;
        self.land(landing);
    }

    /// Where a finished exchange leaves the person.
    fn land(&mut self, landing: keep::Landing) {
        match landing {
            keep::Landing::KeyMenu | keep::Landing::Settings => self.back(),
            // A wrong PIN leaves the pad where it is, with the attempts
            // left on its caption line.
            keep::Landing::Stay => {}
            keep::Landing::Removed => {
                // From the lock screen the keys in memory go with it:
                // the PIN that would unseal them is the one nobody
                // typed (§16.63).
                if self.session.is_locked() {
                    self.wipe_secrets();
                }
                self.keep.clear();
                self.stack.clear();
                self.stack.push(Screen::Home);
                self.screen = Screen::KeptRemoved;
                self.entered();
            }
            keep::Landing::Empty => {
                self.keep.clear();
                self.wipe_secrets();
                self.go_home();
            }
        }
    }

    /// An exchange the shell could not complete: nothing is kept or
    /// opened, and the screen the flow started from comes back. There
    /// is nothing to tell the person that the screen does not already
    /// show (§2.1).
    fn keep_failed(&mut self) {
        let step = self.keep.step.take();
        self.keep.clear();
        match step {
            Some(keep::Step::Keeping) => {
                // A blob that was never stored is not one the device
                // keeps; what was set up for it goes.
                self.kept_blob = Vec::new();
                self.kept_kek = None;
                self.back();
            }
            // The lock screen has nowhere to go back to; the pad stays.
            Some(keep::Step::Opening | keep::Step::Duress) if !self.session.is_locked() => {
                self.back()
            }
            _ => {}
        }
    }

    /// Loads every key the record held, as typed ones load: the words,
    /// their seed, the master key and the backup flag. The PIN that
    /// opened them becomes the session PIN. `false` when nothing loaded.
    fn load_kept(&mut self, keys: osk_keep::KeptKeys) -> bool {
        // The PIN that opened the blob becomes the session PIN, and it
        // is sealed or not held at all, so it is set before anything is
        // loaded: a session key that cannot seal it loads nothing.
        if keys.is_empty() || !self.session.set_pin(self.keep.entry()) {
            return false;
        }
        let mut loaded = 0;
        for kept in keys.iter() {
            // The passphrase is not in the blob, so what comes back is
            // the words' own key; a passphrase wallet is loaded again
            // behind what the person types. A SLIP-39 key comes back as
            // its master secret, which is the seed itself, and a
            // codex32 key as its seed with the tag that says so.
            let (seed, mnemonic, codex32) = match kept.mnemonic() {
                Some(bytes) => {
                    let language = Language::ALL
                        .get(usize::from(bytes.language))
                        .copied()
                        .unwrap_or(Language::English);
                    let count = usize::from(bytes.len).min(MAX_WORDS);
                    let Ok(mnemonic) = Mnemonic::from_indices(language, &bytes.words[..count])
                    else {
                        continue;
                    };
                    let Ok(seed) = mnemonic.to_seed(b"") else {
                        continue;
                    };
                    let Some(seed) = SeedBytes::new(seed.expose()) else {
                        continue;
                    };
                    (Secret::new(seed), Some(mnemonic), false)
                }
                None => {
                    let osk_keep::KeptSecret::MasterSecret { secret, codex32 } = &kept.secret
                    else {
                        continue;
                    };
                    let Some(seed) = SeedBytes::new(secret.as_bytes()) else {
                        continue;
                    };
                    (Secret::new(seed), None, *codex32)
                }
            };
            let master = MasterKey::from_seed_bytes(&seed, self.network);
            let key = LoadedKey::new(seed, master, mnemonic, false, kept.backup_verified);
            let mut key = if codex32 { key.codex32() } else { key };
            if self.keys.iter().any(|k| k.fingerprint == key.fingerprint) {
                continue;
            }
            key.seal(self.session.key_mut());
            self.keys.push(key);
            loaded += 1;
        }
        loaded > 0
    }

    // ----- self-test -----

    /// Runs the self-test. On failure the blocking screen replaces
    /// everything until Exit.
    fn run_selftest(&mut self) {
        #[cfg(test)]
        if self.fail_selftest {
            self.selftest = Some(Err("injected failure"));
            return;
        }
        self.selftest = Some(osk_selftest::run());
    }

    // ----- input -----

    fn apply(&mut self, action: Action) {
        // §16.50: on the small panel a candidate cell is 24 px and a
        // fingertip covers it, so a tap selects and the next tap
        // accepts. The class decides it, and it is what the wizard acts
        // under whatever the action is.
        let two_tap = self.class() == SizeClass::Small;
        match &mut self.wizard {
            Some(Wizard::Load(w)) => w.set_two_tap(two_tap),
            Some(Wizard::Create(w)) => w.set_two_tap(two_tap),
            _ => {}
        }
        if self.screen == Screen::WordList {
            self.word_list.search_mut().set_two_tap(two_tap);
        }
        if self.selftest_failed() {
            if action == Action::Tap(ids::SELFTEST_EXIT) {
                self.exit();
            }
            return;
        }
        if self.boot_refused() {
            if action == Action::Tap(ids::BOOT_EXIT) {
                self.exit();
            }
            return;
        }
        // The lock screen and the stored key's pad have nothing behind
        // them, so Back there is the way out of the app (§16.73).
        if self.session.is_locked() {
            if action == Action::Back {
                self.back_at_root();
            } else if let Action::KeyboardInput(id, input) = action
                && id == ids::LOCK_KEYBOARD
            {
                self.lock_input(input);
            }
            return;
        }
        if self.front_door() {
            if action == Action::Back {
                self.back_at_root();
            } else if let Action::KeyboardInput(id, input) = action
                && id == ids::KEEP_PIN_KEYBOARD
            {
                self.stored_key_input(input);
            }
            return;
        }
        // The eye's reveal belongs to one look at one step of one
        // screen. A page turn keeps it; a step, a section or a screen
        // ends it, so it never survives leaving (UX review, §2.7).
        if action == Action::Tap(ids::SECRET_EYE) {
            self.toggle_reveal();
            self.render();
            return;
        }
        let scope = self.reveal_scope();
        match action {
            Action::Back => self.back(),
            Action::Tap(id) => self.tap(id),
            Action::HoldCompleted(id) => self.hold(id),
            Action::Candidate(id, n) => {
                let class = self.class();
                let network = self.network;
                let second = id == ids::LOAD_CANDIDATES_2;
                if let Some(Wizard::Load(w)) = &mut self.wizard {
                    candidate_tap(w, class, second, n);
                } else if let Some(w) = self.gatherer_mut() {
                    // §16.123 rule 1: a Seed XOR part is typed into the
                    // Create wizard's own entry, on the same strip,
                    // whichever flow holds that wizard.
                    w.candidate(class, second, n, network);
                } else if self.screen == Screen::WordList {
                    let second = id == ids::WORDLIST_CANDIDATES_2;
                    candidate_tap(self.word_list.search_mut(), class, second, n);
                    self.word_search_settled();
                }
            }
            Action::KeyboardInput(id, input) => self.keyboard(id, input),
            Action::Scrolled(_) | Action::PageChanged(..) | Action::Redraw => {}
        }
        if self.reveal_scope() != scope {
            self.reveal_until = None;
        }
    }

    /// §4.2: a value row opens the Choice screen for its list, with the
    /// check on the value the screen already shows.
    fn open_script_choice(&mut self, current: ScriptType) {
        let i = ScriptType::ALL
            .iter()
            .position(|s| *s == current)
            .unwrap_or(0);
        self.overlay = Some(Overlay::Choice(Picker::Script, i));
    }

    /// What the export screen under the Choice is exporting. A Choice
    /// sits over the screen that opened it, so the screen is where the
    /// owner is read from.
    pub(crate) fn export_owner(&self) -> Exported {
        match self.screen {
            Screen::KeyExport(key, account, _) => Exported::Account(key, account),
            Screen::Export(owner) => Exported::Wallet(owner),
            _ => Exported::Wallet(WalletRef::Typed),
        }
    }

    /// The format the export menu has checked.
    pub(crate) fn export_format(&self) -> ExportFormat {
        ExportFormat::ALL[self.export.format.min(ExportFormat::ALL.len() - 1)]
    }

    /// The string the export shows: the key's account in one of its
    /// three forms, or the wallet as a descriptor or as its policy.
    /// The addresses the list on screen shows, for a key or a wallet.
    pub(crate) fn addresses_of(&self, owner: WalletRef) -> Vec<String> {
        let d = &self.detail;
        match owner {
            WalletRef::Key(key) => match self.keys.get(key) {
                Some(k) => self
                    .cached_addresses(k.fingerprint, d.script, d.change)
                    .to_vec(),
                None => Vec::new(),
            },
            WalletRef::Policy(wallet) => match self.wallets.get(wallet) {
                Some(_) => self.wallet_addresses(wallet, d.change).to_vec(),
                None => Vec::new(),
            },
            WalletRef::Typed => match self.typed_fingerprint() {
                Some(fp) => self.cached_addresses(fp, d.script, d.change).to_vec(),
                None => Vec::new(),
            },
        }
    }

    pub(crate) fn export_value(&self, owner: Exported) -> Option<String> {
        let format = self.export_format();
        let owner = match owner {
            // §16.110 rule 1: a key exports its public accounts, each
            // with the origin every coordinator asks for. BIP 129's key
            // record is written once, where its two values were typed.
            Exported::Account(..) => {
                return match (format, self.account_view.as_ref()?) {
                    (ExportFormat::BsmsSigner, _) => self.bsms.record.clone(),
                    (ExportFormat::Slip132, AccountView::Single(a)) => Some(a.slip132_string()),
                    (_, AccountView::Single(a)) => {
                        Some(origin_key(a.master_fingerprint(), a.path(), a.xpub()))
                    }
                    (_, AccountView::Multi(a)) => {
                        Some(origin_key(a.master_fingerprint(), a.path(), a.xpub()))
                    }
                };
            }
            Exported::Wallet(owner) => owner,
        };
        match owner {
            WalletRef::Typed | WalletRef::Key(_) => None,
            WalletRef::Policy(wallet) => {
                // §16.113: a silent payments wallet's three forms. The
                // scan descriptor is drawn on a Secret screen and never
                // reaches this function.
                if self.silent_of(wallet).is_some() {
                    let address = self.silent_address_text(wallet, None)?;
                    return match format {
                        ExportFormat::SilentUri => osk_bip::silent::uri(&address)
                            .ok()
                            .map(|u| String::from(u.as_str())),
                        ExportFormat::SilentDns => {
                            if !self.dns.ready() {
                                return None;
                            }
                            osk_bip::silent::dns_record(
                                self.dns.user.trim(),
                                self.dns.domain.trim(),
                                &address,
                            )
                            .ok()
                            .map(|r| String::from(r.as_str()))
                        }
                        _ => None,
                    };
                }
                let policy = self.wallets.get(wallet)?;
                // §16.114: the import file is the wallet's multipath
                // descriptor whatever else the wallet could be handed
                // over as, so it comes before the account forms.
                if format == ExportFormat::CoreImport {
                    return Some(osk_bip::core_import::import_descriptors(
                        &policy.to_descriptor_checksummed(),
                        self.rescan,
                    ));
                }
                // A wallet over one key this device holds is still one
                // account, so the two account-key forms are what a
                // coordinator asks for; every other wallet has several
                // keys and only a descriptor states them.
                if format == ExportFormat::Bsms {
                    return self.bsms_record(policy).map(|r| r.to_text());
                }
                if let Some(account) = self.single_account(policy) {
                    return Some(match format {
                        ExportFormat::Xpub => account.xpub_string(),
                        ExportFormat::Slip132 => account.slip132_string(),
                        _ => policy.to_descriptor_checksummed(),
                    });
                }
                Some(match format {
                    ExportFormat::Policy => policy.to_text(),
                    _ => policy.to_descriptor_checksummed(),
                })
            }
        }
    }

    /// BIP 129's descriptor record for `policy`: the wallet's template,
    /// the two chains this device derives, and the wallet's own first
    /// receive address, which is what every signer checks the record
    /// against.
    pub(crate) fn bsms_record(&self, policy: &WalletPolicy) -> Option<bsms::DescriptorRecord> {
        let network = self.network;
        let first = policy.address_at(network, false, 0).ok()?;
        Some(bsms::DescriptorRecord {
            policy: policy.clone(),
            paths: alloc::vec![String::from("/0/*"), String::from("/1/*")],
            first_address: alloc::format!("{first}"),
            network,
        })
    }

    /// The account xpub behind a single-sig wallet whose one key is a
    /// loaded key's, if it is one.
    pub(crate) fn single_account(&self, policy: &WalletPolicy) -> Option<&AccountXpub> {
        let Template::Single { script } = policy.template() else {
            return None;
        };
        let fp = policy.keys().first()?.fingerprint()?;
        self.cached_account(fp, script)
    }

    /// §4.9: "The wallet export animates on the panel like any other
    /// payload." The row opens the QR page in the shape a camera can
    /// read: one code where its modules clear the pitch floor at the
    /// class's side, the BC-UR parts otherwise, with the toggle dimmed
    /// and the reason "too dense" as the transaction's is.
    fn show_export_qr(&mut self, owner: Exported) {
        self.export.qr = true;
        self.export.version = codes::max_version(self.qr_side_px(), self.dpi);
        let scans = self
            .export_value(owner)
            .is_some_and(|v| codes::fits_one_code(v.as_bytes(), self.export.version));
        self.export.scans = scans;
        self.set_export_qr_mode(owner, !scans);
    }

    /// Switches the wallet export's QR between one code and the animated
    /// parts, and starts the run when it is the parts.
    fn set_export_qr_mode(&mut self, owner: Exported, animated: bool) {
        self.export.animated = animated;
        self.export.run = None;
        if !animated {
            return;
        }
        let now = self.now_ms;
        let version = self.export.version;
        if let Some(value) = self.export_value(owner) {
            self.export.run =
                codes::UrRun::new(codes::UrKind::Bytes, value.as_bytes(), version, now);
        }
    }

    /// What the Bitcoin Core import file is offered as: the wallet's
    /// name where it has one, and its descriptor checksum where it does
    /// not (`docs/PLANNING.md` §16.114).
    pub(crate) fn core_import_file_name(&self) -> String {
        let policy = match self.export_owner() {
            Exported::Wallet(WalletRef::Policy(w)) => self.wallets.get(w),
            _ => None,
        };
        let what = match policy {
            Some(policy) => match self.names.policy_name(policy) {
                Some(name) => String::from(name),
                None => policy.checksum(),
            },
            None => String::new(),
        };
        strings::fill1(self.strings().core_import_file_name, &what)
    }

    /// The Save on that file: the bytes go to the shell, and the row
    /// under it waits for the answer.
    fn save_core_import(&mut self, owner: Exported) {
        if self.export_format() != ExportFormat::CoreImport {
            return;
        }
        let Some(value) = self.export_value(owner) else {
            return;
        };
        self.export_save = sign::Save::Waiting;
        let name_hint = self.core_import_file_name();
        self.commands.push_back(Command::WriteFile {
            kind: FileKind::Any,
            name_hint,
            bytes: value.into_bytes(),
        });
    }

    /// §4.5: the export string's reference row opens the Compare screen,
    /// which shows a descriptor as structure and anything else whole.
    fn open_export_compare(&mut self, owner: Exported) {
        let format = self.export_format();
        let Some(value) = self.export_value(owner) else {
            return;
        };
        self.overlay = Some(Overlay::Compare(Comparison {
            title: String::from(views::export::format_name(format, self.strings())),
            label: None,
            value,
            descriptor: format == ExportFormat::Descriptor,
        }));
    }

    /// One Compare screen for a long string, wherever its reference row
    /// was (§4.5).
    fn open_compare(&mut self, title: &str, value: impl Into<String>) {
        self.overlay = Some(Overlay::Compare(Comparison {
            title: String::from(title),
            label: None,
            value: value.into(),
            descriptor: false,
        }));
    }

    /// The same screen for one of a run: `title` is the kind and `label`
    /// the instance, which §5 Compare puts over the string.
    fn open_compare_one(&mut self, title: &str, label: String, value: impl Into<String>) {
        self.overlay = Some(Overlay::Compare(Comparison {
            title: String::from(title),
            label: Some(label),
            value: value.into(),
            descriptor: false,
        }));
    }

    /// The same screen for a descriptor, which §4.5 compares as
    /// structure rather than as a run of characters.
    fn open_compare_descriptor(&mut self, title: &str, value: impl Into<String>) {
        self.overlay = Some(Overlay::Compare(Comparison {
            title: String::from(title),
            label: None,
            value: value.into(),
            descriptor: true,
        }));
    }

    /// The Choice and Compare screens a row opened: the check moves on a
    /// tap, Continue applies it, and Done closes the string.
    fn tap_overlay(&mut self, id: Id) {
        if id == ids::COMPARE_DONE {
            self.overlay = None;
            return;
        }
        // §4.5: the key inside a descriptor is a reference value, and it
        // opens the extended key on a Compare screen of its own.
        if id == ids::COMPARE_KEY {
            let key = match self.screen {
                // A wallet over one key has one account key, which is
                // what the structure opens; a wallet of several names
                // no one key the structure could open.
                Screen::Export(WalletRef::Policy(w)) => self
                    .wallets
                    .get(w)
                    .and_then(|p| self.single_account(p))
                    .map(AccountXpub::xpub_string),
                Screen::Export(WalletRef::Key(_)) => None,
                Screen::Inspect => self
                    .inspect
                    .as_ref()
                    .and_then(|d| osk_ui::descriptor::key(&d.text)),
                _ => None,
            };
            if let Some(value) = key {
                let label = self.strings().export_xpub;
                self.open_compare(label, value);
            }
            return;
        }
        // §5 Menu, "Files": the row is the answer. The list closes and
        // the screen that asked is back, still waiting, so the bytes
        // land where the request came from.
        if let Some(Overlay::Files(list)) = &self.overlay {
            if let Some(i) = ids::index_in(id, ids::FILES_ROW_BASE, list.entries.len()) {
                let kind = list.kind;
                let name = list.entries[i].name.clone();
                self.overlay = None;
                self.commands.push_back(Command::ReadFile { kind, name });
                self.ui.set_scroll(ids::SCROLL, 0);
            }
            return;
        }
        // §4.2 Setting: the tap applies at once and the screen stays.
        if let Some(Overlay::Setting(setting)) = &self.overlay {
            self.apply_setting(*setting, id);
            return;
        }
        let Some(Overlay::Choice(picker, picked)) = &mut self.overlay else {
            return;
        };
        let picker = *picker;
        if let Some(i) = ids::index_in(id, ids::PICK_BASE, picker.len()) {
            *picked = i;
            return;
        }
        if id != ids::PICK_CONTINUE {
            return;
        }
        let picked = *picked;
        self.overlay = None;
        match (picker, self.screen) {
            (Picker::Script, Screen::Addresses(_)) => {
                self.detail = DetailState {
                    script: ScriptType::ALL[picked],
                    change: self.detail.change,
                    ..DetailState::default()
                };
            }
            (Picker::Script, Screen::Export(_)) => {
                self.export.script = ScriptType::ALL[picked];
                // A format the new script type has no form of cannot stay
                // picked: the format row would name a format the Choice
                // no longer offers, and the Choice would open with nothing
                // chosen. SLIP-132 on Taproot falls back to the account
                // key, which for Taproot is the string SLIP-132 would have
                // produced anyway.
                let format = self.export_format();
                let owner = self.export_owner();
                if !self.format_offered(format, owner, self.export.script) {
                    self.export.format = ExportFormat::ALL
                        .iter()
                        .position(|f| *f == ExportFormat::Xpub)
                        .expect("the account key is always a format");
                }
            }
            (Picker::XpubScript, _) => self.xpub_wallet(ScriptType::ALL[picked]),
            (Picker::Script, Screen::SignMessage) => {
                if let Some(flow) = self.message.as_mut() {
                    flow.set_script(ScriptType::ALL[picked]);
                }
            }
            // §16.110 rule 1: the account is derived once, here, and
            // the Export screen reads it from there.
            (Picker::Account, Screen::KeyDetail(key)) => {
                let account = Account::ALL[picked];
                self.account_view = self.derive_account(key, account);
                self.push(Screen::KeyExport(key, account, KeyExportStep::Export));
            }
            // §16.114: Words is the child-key flow, which adds a key;
            // every other application derives a value to transcribe.
            (Picker::Bip85App, Screen::KeyDetail(key)) => {
                let app = Bip85App::ALL[picked];
                if app == Bip85App::Words {
                    self.picked_key = Some(key);
                    self.push(Screen::OpenChild(key));
                    self.child = Some(ChildFlow::new());
                } else if app == Bip85App::Hex {
                    self.bip85 = Some(Bip85Flow::new(app));
                    self.overlay = Some(Overlay::Choice(Picker::Bip85Bytes, 0));
                } else {
                    self.bip85 = Some(Bip85Flow::new(app));
                    self.push(Screen::Bip85(key));
                }
            }
            (Picker::Bip85Bytes, Screen::KeyDetail(key)) => {
                if let Some(flow) = self.bip85.as_mut() {
                    flow.bytes = HEX_BYTE_COUNTS[picked];
                }
                self.push(Screen::Bip85(key));
            }
            // §16.114: where Core starts scanning is the one question
            // the import file asks, and it is asked before the file is
            // shown.
            // §16.113: the form the address on screen is shown in.
            (Picker::SilentForm, _) => {
                if let Some(form) = silent::AddressForm::ALL.get(picked) {
                    self.silent_form = *form;
                }
            }
            (Picker::Rescan, _) => {
                self.rescan = if picked == 0 {
                    osk_bip::core_import::Rescan::Start
                } else {
                    osk_bip::core_import::Rescan::Now
                };
            }
            (Picker::Format, _) => {
                self.export.format = picked;
                self.export_save = sign::Save::Idle;
                // §16.113: the record is published under a user name
                // and a domain, which are asked for before it is
                // written, as BIP 129's key record asks for its two
                // values.
                if self.export_format() == ExportFormat::SilentDns
                    && let Screen::Export(WalletRef::Policy(wallet)) = self.screen
                {
                    self.dns = silent::Dns::default();
                    self.push(Screen::SilentDns(wallet));
                }
                // §16.114: the import file is written from a rescan
                // point, which the Choice asks for before the file is
                // shown, as the key record asks for its two values.
                if self.export_format() == ExportFormat::CoreImport {
                    self.overlay = Some(Overlay::Choice(Picker::Rescan, 0));
                }
                // BIP 129's key record is a flow and not a format: it
                // asks for the session token and the description first.
                if let Screen::KeyExport(key, account, _) = self.screen
                    && self.export_format() == ExportFormat::BsmsSigner
                {
                    let description = match self.keys.get(key) {
                        Some(k) => text::fingerprint_hex(k.fingerprint),
                        None => String::new(),
                    };
                    self.bsms = BsmsEntry {
                        description,
                        ..BsmsEntry::default()
                    };
                    self.screen = Screen::KeyExport(key, account, KeyExportStep::Token);
                    self.ui.set_scroll(ids::SCROLL, 0);
                }
            }
            (Picker::SearchBy, Screen::WordList) => match wordlist::SearchBy::ALL.get(picked) {
                Some(by) => self.word_list.set_by(*by),
                None => self.push(Screen::Word(0)),
            },
            (Picker::ReadAs, Screen::Tool(_)) => {
                if let Some(r) = osk_codec::encodings::ReadAs::ALL.get(picked) {
                    self.calc.set_read_as(*r);
                }
            }
            (Picker::FromUnit, Screen::Tool(_)) => {
                if let Some(u) = osk_ui::components::Denomination::ALL.get(picked) {
                    self.calc.set_from(*u);
                }
            }
            (Picker::PolicyScript, Screen::Tool(_)) => {
                if let Some(script) = osk_bip::compile::PolicyScript::ALL.get(picked) {
                    self.calc.set_script(*script);
                }
            }
            // The chosen key's account xpub becomes the field, and the
            // answer opens: the tool has everything it needs.
            (Picker::ConvertKey(_), Screen::Tool(Tool::ConvertKey)) => {
                if let Some(xpub) = self.account_xpub_of(picked) {
                    self.calc.set(Tool::ConvertKey, xpub);
                    if self.calc.ready(
                        Tool::ConvertKey,
                        self.network,
                        &self.policy_key_expressions(),
                    ) {
                        self.push(Screen::ToolResult(Tool::ConvertKey));
                    }
                }
            }
            (Picker::MessageKey(_), Screen::SignMessage) => {
                if let Some(flow) = self.message.as_mut() {
                    flow.set_key(picked);
                }
            }
            // "Keep this wallet on the device?": "No" is what the wallet
            // already is, so only "Yes" writes it (§16.104 rule 7).
            (Picker::KeepWallet, Screen::Wallet(WalletRef::Policy(w))) => {
                if picked == 1 {
                    self.set_wallet_kept(w, true);
                }
            }
            // §16.116: an LND cipher seed is typed, a loaded key is
            // chosen; with one such key there is nothing to choose.
            (Picker::LightningFrom, _) => {
                if picked == 0 {
                    self.lightning = Some(Lightning::aezeed());
                    self.push(Screen::Lightning);
                    self.start_aezeed_entry();
                } else if self.lightning_keys() == 1 {
                    self.open_lightning_key(0);
                } else if self.lightning_keys() > 1 {
                    self.overlay = Some(Overlay::Choice(Picker::LightningKey(self.keys.len()), 0));
                }
            }
            (Picker::LightningKey(_), _) => self.open_lightning_key(picked),
            // §16.117: which dial the counter turns, over a key's page.
            (Picker::VanityDial, Screen::KeyDetail(key)) => {
                if let Some(dial) = vanity::Dial::ALL.get(picked) {
                    self.start_vanity(key, *dial);
                }
            }
            (Picker::MessageFormat, _) => {
                if let Some(flow) = self.message.as_mut() {
                    flow.set_format(message::FORMATS[picked]);
                }
            }
            _ => {}
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The eight settings a restart restores (§6).
    fn settings(&self) -> settings::Settings {
        settings::Settings {
            network: self.network,
            unit: self.unit,
            lock_after_ms: self.session.lock_after_ms(),
            wipe_after_ms: self.session.wipe_after_ms(),
            scramble_pin: self.session.scramble_pin(),
            camera_rotation: self.camera_rotation(),
            nonce: self.nonce,
            schnorr: self.schnorr,
            first_run_done: self.first_run_done,
            backup_memory_kib: self.backup_memory,
        }
    }

    /// Applies what the shell kept last time. The timers go in the order
    /// a person would have tapped them, the auto-wipe last, so that a
    /// file whose wipe is shorter than its lock comes out as consistent
    /// as the Settings screen makes it ([`session::Session`]).
    fn apply_settings(&mut self, settings: &settings::Settings) {
        self.set_network(settings.network);
        self.unit = settings.unit;
        self.camera_rotation = settings.camera_rotation;
        self.nonce = settings.nonce;
        self.schnorr = settings.schnorr;
        self.session.set_lock_after(settings.lock_after_ms);
        self.session.set_wipe_after(settings.wipe_after_ms);
        self.session.set_scramble_pin(settings.scramble_pin);
        self.first_run_done = settings.first_run_done;
        self.backup_memory = settings.backup_memory_kib;
    }

    /// Asks the shell to keep the settings as they now are. Only a user
    /// changing one on the Settings screen reaches here, and a Tier D
    /// device persists nothing at all (§3), so it asks for nothing.
    fn store_settings(&mut self) {
        if self.tier == AssuranceTier::D {
            return;
        }
        let bytes = settings::write(&self.settings());
        self.commands.push_back(Command::StoreSettings { bytes });
    }

    /// The first run is over: the Start here document is not the screen
    /// a device with nothing loaded opens on any more. A Tier D device
    /// persists nothing (§3), so there it comes back every run.
    fn finish_first_run(&mut self) {
        self.first_run_addresses = false;
        if !self.first_run_done {
            self.first_run_done = true;
            self.store_settings();
        }
    }

    /// §4.2 Setting: the row that was tapped becomes the setting's
    /// value, and the Choice stays on screen with the check moved. The
    /// new value is also what the device comes back to (§6).
    fn apply_setting(&mut self, setting: Setting, id: Id) {
        let changed = match setting {
            Setting::Network => ids::index_in(id, ids::SETTINGS_NET_BASE, Network::ALL.len())
                .map(|i| self.set_network(Network::ALL[i]))
                .is_some(),
            Setting::Unit => ids::index_in(id, ids::SETTINGS_UNIT_BASE, UNITS.len())
                .map(|i| self.unit = UNITS[i])
                .is_some(),
            Setting::CameraRotation => ids::index_in(
                id,
                ids::SETTINGS_CAMERA_ROTATION_BASE,
                scan::CAMERA_ROTATIONS.len(),
            )
            .map(|i| self.camera_rotation = scan::CAMERA_ROTATIONS[i])
            .is_some(),
            Setting::Nonce => ids::index_in(id, ids::SETTINGS_NONCE_BASE, NONCES.len())
                .map(|i| self.nonce = NONCES[i])
                .is_some(),
            Setting::Schnorr => ids::index_in(id, ids::SETTINGS_SCHNORR_BASE, SCHNORRS.len())
                .map(|i| self.schnorr = SCHNORRS[i])
                .is_some(),
            Setting::BackupMemory => {
                ids::index_in(id, ids::SETTINGS_BACKUP_MEMORY_BASE, BACKUP_MEMORY.len())
                    .map(|i| self.backup_memory = Some(BACKUP_MEMORY[i]))
                    .is_some()
            }
            Setting::Lock => ids::index_in(
                id,
                ids::SETTINGS_LOCK_AFTER_BASE,
                session::LOCK_OPTIONS.len(),
            )
            .map(|i| self.session.set_lock_after(session::LOCK_OPTIONS[i]))
            .is_some(),
            Setting::Wipe => ids::index_in(
                id,
                ids::SETTINGS_WIPE_AFTER_BASE,
                session::WIPE_OPTIONS.len(),
            )
            .map(|i| self.session.set_wipe_after(session::WIPE_OPTIONS[i]))
            .is_some(),
        };
        if changed {
            self.store_settings();
        }
    }

    /// The key list's controls, which Keys and `Wide` Home share.
    /// The ways a key arrives, which Keys' "Add a key" opens and which
    /// its empty state offers directly.
    fn tap_add(&mut self, id: Id) {
        if id == ids::KEYS_LOAD {
            self.start_load();
        } else if id == ids::KEYS_CREATE {
            self.start_create();
        } else if id == ids::ADD_CREATE_SLIP39 {
            self.start_create_shares();
        } else if id == ids::ADD_CREATE_CODEX32 {
            self.start_create_codex32();
        } else if id == ids::ADD_OPEN_PASSPHRASE
            && self.key_was_asked_for()
            && self.passphrase_sources().next().is_some()
        {
            self.push(Screen::PickKey(PickFor::Passphrase));
        } else if id == ids::ADD_OPEN_CHILD && self.key_was_asked_for() && !self.keys.is_empty() {
            self.push(Screen::PickKey(PickFor::Child));
        }
    }

    /// Whether a wallet's or a transaction's row sent the person here
    /// for one key in particular (§16.104 rule 6), which is the one
    /// question Add a key answers with more than the ways a key is
    /// made.
    pub(crate) fn key_was_asked_for(&self) -> bool {
        self.add_key_return.is_some()
    }

    /// The loaded keys a passphrase can be opened over: the ones whose
    /// words this device holds.
    pub(crate) fn passphrase_sources(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.keys.len()).filter(|i| self.keys[*i].has_mnemonic())
    }

    /// "Which key?": the key at `i` was chosen, so the flow that asked
    /// goes on.
    fn pick_key(&mut self, purpose: PickFor, i: usize) {
        if self.keys.get(i).is_none() {
            return;
        }
        self.picked_key = Some(i);
        match purpose {
            PickFor::Passphrase => {
                if self.keys[i].has_mnemonic() {
                    self.push(Screen::OpenPassphrase(i));
                    self.open_finish = Some(Finish::new());
                }
            }
            PickFor::Child => {
                self.push(Screen::OpenChild(i));
                self.child = Some(ChildFlow::new());
            }
        }
    }

    /// The two ways a wallet arrives, which Wallets' "Add a wallet"
    /// opens: the one wizard for all four kinds, and the scanner
    /// (§16.104 rule 2).
    fn tap_add_wallet(&mut self, id: Id) {
        if id == ids::BUILD_NEW {
            self.start_build();
        } else if id == ids::WALLETS_LOAD {
            self.start_scan(Expect::Wallet);
        }
    }

    /// "Script type" on the single-sig flow: the wallet the chosen key
    /// and script make, as its own review, which ends in "Add this
    /// wallet" like every other wallet's (§16.104 rule 2).
    /// Home: the six tiles, and nothing else. Home never changes shape
    /// (§16.104 rule 4).
    fn tap_home(&mut self, id: Id) {
        // While Home is drawing the first run's document, its one
        // action is the way on.
        if self.start_here_shown() {
            if id == ids::START_HERE_CONTINUE {
                self.push(Screen::Keys);
                self.push(Screen::Add);
            }
            return;
        }
        if let Some(t) = ids::index_in(id, ids::HOME_TILE_BASE, HUB_TILES.len()) {
            self.open_tile(t);
        }
    }

    /// Keys: a key's row and the bottom action.
    fn tap_keys(&mut self, id: Id) {
        if id == ids::KEYS_ADD {
            self.push(Screen::Add);
        } else if let Some(i) = ids::index_in(id, ids::KEYS_ROW_BASE, self.keys.len()) {
            self.push(Screen::KeyDetail(i));
        } else {
            // The empty state's two rows start the flows they name.
            self.tap_add(id);
        }
    }

    /// Wallets: a wallet's row, and the bottom action.
    fn tap_wallets(&mut self, id: Id) {
        if id == ids::WALLETS_ADD {
            self.push(Screen::AddWallet);
        } else if id == ids::WALLETS_LOAD {
            self.start_scan(Expect::Wallet);
        } else if let Some(i) = ids::index_in(id, ids::WALLETS_POLICY_ROW_BASE, self.wallets.len())
        {
            self.push(Screen::Wallet(WalletRef::Policy(i)));
        }
    }

    /// Whether nothing at all is loaded, which is what the first run
    /// hands its document to.
    pub(crate) fn home_is_empty(&self) -> bool {
        self.keys.is_empty() && self.wallets.is_empty()
    }

    /// Whether `wallet` signs messages, which only a wallet over one
    /// key does: a message is signed by one key (§16.104 rule 2).
    pub(crate) fn wallet_signs_messages(&self, wallet: WalletRef) -> bool {
        match wallet {
            WalletRef::Policy(w) => self
                .wallets
                .get(w)
                .is_some_and(|p| matches!(p.template(), Template::Single { .. })),
            _ => false,
        }
    }

    /// The members of the wallet at `w`: what each is called, and the
    /// loaded key it is, where this device holds one. A member with no
    /// origin names no master, so it is nobody's. A FROST wallet's
    /// members are its record's public shares, matched against the
    /// loaded keys (§16.104 rule 3).
    pub(crate) fn wallet_keys(&self, w: usize) -> Vec<(String, Option<usize>)> {
        let s = self.strings();
        let Some(policy) = self.wallets.get(w) else {
            return Vec::new();
        };
        if policy.record().is_some() {
            return self.record_members(w);
        }
        // §16.113: a silent payments wallet's one member is the key its
        // two halves were derived from, which the record names by its
        // origin.
        if let Some(record) = policy.silent() {
            let fp = record.fingerprint;
            return alloc::vec![(
                text::fingerprint_hex(fp),
                self.keys.iter().position(|k| k.fingerprint == fp),
            )];
        }
        policy
            .keys()
            .iter()
            .map(|pk| match pk.fingerprint() {
                Some(fp) => (
                    text::fingerprint_hex(fp),
                    self.keys.iter().position(|k| k.fingerprint == fp),
                ),
                None => (String::from(s.wallet_origin_unknown), None),
            })
            .collect()
    }

    /// The account view a key's Export shows, derived once (§16.110
    /// rule 1).
    fn derive_account(&self, key: usize, account: Account) -> Option<AccountView> {
        let master = self.keys.get(key)?.master.as_ref()?;
        match account {
            Account::Single(script) => master.account_xpub(script, 0).ok().map(AccountView::Single),
            Account::Multi(script) => master
                .multisig_account_xpub(script, 0)
                .ok()
                .map(AccountView::Multi),
        }
    }

    /// Whether this key has a master to derive accounts from, which is
    /// what the "Account key" row needs (§16.110 rule 1).
    pub(crate) fn key_has_master(&self, key: usize) -> bool {
        self.keys.get(key).is_some_and(|k| k.master.is_some())
    }

    /// The keys a transaction names, for its own Keys review (§16.110
    /// rule 3): the member each row stands for, what the row is called,
    /// and the loaded key it is, where this device holds one.
    ///
    /// A multisig transaction names its cosigners in script order; any
    /// other origin the inputs state follows them. A transaction that
    /// spends a FROST group names its members by public share, as the
    /// group's own review does.
    pub(crate) fn transaction_keys(
        &self,
        reading: bool,
    ) -> Vec<(Option<Member>, String, Option<usize>)> {
        let flow = if reading { &self.decode } else { &self.sign };
        let Some(insp) = flow.inspection() else {
            return Vec::new();
        };
        if let Some(t) = insp.threshold.as_ref() {
            return self
                .wallet_keys(t.wallet)
                .into_iter()
                .enumerate()
                .map(|(i, (label, loaded))| (self.wallet_member(t.wallet, i), label, loaded))
                .collect();
        }
        let mut fingerprints: Vec<Fingerprint> = Vec::new();
        if let Some(m) = insp.multisig.as_ref() {
            for c in &m.cosigners {
                if let Some(fp) = c.fingerprint
                    && !fingerprints.contains(&fp)
                {
                    fingerprints.push(fp);
                }
            }
        }
        for input in &insp.inputs {
            if let Some(o) = input.origin.as_ref()
                && !fingerprints.contains(&o.fingerprint)
            {
                fingerprints.push(o.fingerprint);
            }
        }
        fingerprints
            .into_iter()
            .map(|fp| {
                (
                    Some(Member::Master(fp)),
                    text::fingerprint_hex(fp),
                    self.keys.iter().position(|k| k.fingerprint == fp),
                )
            })
            .collect()
    }

    /// The rows of the Export screen a key's account opens, and its two
    /// Entry steps (§16.110 rules 1 and 2).
    fn tap_key_export(&mut self, id: Id, key: usize, account: Account, step: KeyExportStep) {
        if step != KeyExportStep::Export {
            // The token's preset writes the token that says the session
            // is not encrypted.
            if id == ids::BSMS_TOKEN_NONE && step == KeyExportStep::Token {
                self.bsms.token = String::from(bsms::NO_ENCRYPTION);
            }
            return;
        }
        let owner = Exported::Account(key, account);
        if id == ids::EXPORT_FORMAT {
            self.overlay = Some(Overlay::Choice(Picker::Format, self.export.format));
        } else if id == ids::EXPORT_SHOW {
            self.show_export_qr(owner);
        } else if id == ids::EXPORT_ANIMATED {
            let next = !self.export.animated || !self.export.scans;
            self.set_export_qr_mode(owner, next);
        } else if id == ids::EXPORT_TEXT {
            self.open_export_compare(owner);
        }
    }

    /// BIP 129 round 1, written from the account node's own secret key
    /// for the length of one call (§16.110 rule 2). `signer_record`
    /// refuses a secret that is not the account's, so a record that
    /// comes back is a record that key signed.
    fn write_signer_record(&mut self, key: usize, account: Account) {
        self.bsms.record = None;
        let Account::Multi(script) = account else {
            return;
        };
        let token = self.bsms.token.clone();
        let description = self.bsms.description.clone();
        let Some(master) = self.keys.get(key).and_then(|k| k.master.as_ref()) else {
            return;
        };
        self.bsms.record = master
            .with_multisig_account_secret(script, 0, |secret, view| {
                bsms::signer_record(master.secp(), view, secret, &token, &description)
                    .ok()
                    .map(|r| r.to_text())
            })
            .ok()
            .flatten();
    }

    /// What row `i` of `w`'s Keys review names: the member a key added
    /// from that row is compared with (§16.104 rule 6).
    fn wallet_member(&self, w: usize, i: usize) -> Option<Member> {
        let policy = self.wallets.get(w)?;
        match policy.record() {
            Some(record) => record
                .info
                .pubshares
                .get(i)
                .copied()
                .flatten()
                .map(Member::Share),
            None => policy.keys().get(i)?.fingerprint().map(Member::Master),
        }
    }

    /// Whether this device holds a private key of `wallet`, which is
    /// what the Sign rows need.
    pub(crate) fn wallet_can_sign(&self, wallet: WalletRef) -> bool {
        match wallet {
            WalletRef::Typed => false,
            WalletRef::Key(key) => self.keys.get(key).is_some(),
            // A FROST wallet signs with the loaded keys that are members
            // of its group, which `wallet_holds_key` matches by public
            // share (§16.104 rule 3).
            WalletRef::Policy(policy) => self
                .wallets
                .get(policy)
                .is_some_and(|p| self.wallet_holds_key(p)),
        }
    }

    fn tap(&mut self, id: Id) {
        if id == ids::BACK {
            self.back();
            return;
        }
        // §16.105: the app bar's one bridge to Learn. The page goes on
        // the screen stack and nothing else is touched, so the chevron
        // comes back to the step the button was tapped on.
        if id == ids::INFO
            && let Some(target) = self.learn_target()
        {
            self.open_learn_topic(target);
            return;
        }
        // §4.10: Copy is a row on the screens that carry a public
        // string, and it is the same row wherever it is.
        if id == ids::COPY
            && self.has_clipboard
            && let Some(text) = self.copy_value()
        {
            self.commands.push_back(Command::WriteClipboard {
                kind: FileKind::Text,
                text,
            });
            return;
        }
        // §16.134: "Save as PNG" is a row on the QR screens whose code
        // is public, and it is the same row wherever it is.
        if id == ids::SAVE_PNG {
            self.save_png();
            return;
        }
        // The Tier A start screen has one way on, and it goes to Home
        // with nothing behind it.
        if id == ids::NO_SECURE_BOOT_CONTINUE && self.screen == Screen::NoSecureBoot {
            self.go_home();
            return;
        }
        if self.overlay.is_some() {
            self.tap_overlay(id);
            return;
        }
        if self.building_scan() {
            self.tap_scan(id);
            return;
        }
        if self.wizard.is_some() {
            self.tap_wizard(id);
            return;
        }
        // The area controls: Home's tiles below `Wide`, and the `wide`
        // sidebar, which reaches every area from every screen it is live
        // on. A dimmed sidebar has no hit targets to reach.
        if self.class() == SizeClass::Wide
            && !self.sidebar_dimmed()
            && let Some(t) = ids::index_in(id, ids::HOME_TILE_BASE, HUB_TILES.len())
        {
            if self.screen != Screen::Home {
                self.go_home();
            }
            self.open_tile(t);
            return;
        }
        if id == ids::STATUS_TIER {
            self.push(Screen::Tiers);
            return;
        }
        if id == ids::STATUS_LOCK {
            self.lock();
            return;
        }
        match self.screen {
            Screen::Home => self.tap_home(id),
            Screen::Notes => self.tap_notes(id),
            Screen::Note => self.tap_note(id),
            Screen::Sheet(wallet) => self.tap_sheet(id, wallet),
            Screen::SilentAddress(..) => self.tap_silent_address(id),
            Screen::SilentLabels(wallet) => self.tap_silent_labels(id, wallet),
            Screen::SilentCheck(_) => self.tap_silent_check(id),
            // The entry's one control is its keyboard, and the Secret
            // screen's is the eye the app bar draws.
            Screen::SilentDns(_) | Screen::SilentSecret(_) => {}
            Screen::OpenedSheet => self.tap_opened_sheet(id),
            Screen::ExportForm => self.tap_export_form(id),
            Screen::Sealed | Screen::SealedQr => self.tap_sealed(id),
            // The two entries are the keyboard's alone.
            Screen::NoteText | Screen::SealPass => {}
            Screen::StartHere => {
                if id == ids::START_HERE_CONTINUE {
                    self.push(Screen::Add);
                }
            }
            // The key is added already; "Check an address" opens its
            // wallet's list, and leaving that list ends the first run.
            Screen::Created(key) => {
                if id == ids::CREATED_CHECK && self.keys.get(key).is_some() {
                    self.first_run_addresses = true;
                    self.push(Screen::Addresses(WalletRef::Key(key)));
                } else if id == ids::CREATED_DONE {
                    self.go_home();
                }
            }
            Screen::Keys => self.tap_keys(id),
            Screen::Wallets => self.tap_wallets(id),
            Screen::Add => self.tap_add(id),
            Screen::AddWallet => self.tap_add_wallet(id),
            Screen::PickKey(purpose) => {
                if let Some(i) = ids::index_in(id, ids::PICK_KEY_BASE, self.keys.len()) {
                    self.pick_key(purpose, i);
                }
            }
            // A wallet's members: a loaded one opens its key page, and
            // one this device has no key for opens Add a key, which
            // comes back here once a key is loaded (§16.104 rule 6).
            Screen::WalletKeys(w) => {
                if let Some(i) =
                    ids::index_in(id, ids::WALLET_KEY_ROW_BASE, self.wallet_keys(w).len())
                {
                    match self.wallet_keys(w)[i].1 {
                        Some(k) => self.push(Screen::KeyDetail(k)),
                        None => {
                            self.add_key_return = Some(AddKeyReturn {
                                to: ReturnTo::Wallet(w),
                                member: self.wallet_member(w, i),
                            });
                            self.push(Screen::Add);
                        }
                    }
                }
            }
            Screen::Explore => self.tap_explore(id),
            Screen::KeyDetail(key) => {
                if id == ids::DETAIL_ACCOUNT && self.key_has_master(key) {
                    self.overlay = Some(Overlay::Choice(Picker::Account, 0));
                } else if id == ids::DETAIL_BIP85 && self.key_has_master(key) {
                    self.overlay = Some(Overlay::Choice(Picker::Bip85App, 0));
                } else if id == ids::DETAIL_VANITY && self.key_has_master(key) {
                    let first = self.vanity_first_dial(key);
                    self.overlay = Some(Overlay::Choice(Picker::VanityDial, first));
                } else if id == ids::DETAIL_OPEN_PASSPHRASE
                    && self.keys.get(key).is_some_and(LoadedKey::has_mnemonic)
                {
                    // §16.127 rule 3: the passphrase is opened over
                    // this key, which the page is already about.
                    self.picked_key = Some(key);
                    self.push(Screen::OpenPassphrase(key));
                    self.open_finish = Some(Finish::new());
                } else if id == ids::DETAIL_ADD_WALLET && self.key_has_master(key) {
                    self.start_build_from(key);
                } else if id == ids::DETAIL_BACKUP {
                    self.push(Screen::BackupMenu(key));
                } else if id == ids::KEY_FORGET && self.keys.get(key).is_some() {
                    self.push(Screen::Forget(key));
                } else if id == ids::KEEP_ROW && self.keep_offered(key) {
                    self.keep_offer = false;
                    self.push(Screen::Keep(key));
                }
            }
            Screen::Tools => {
                if id == ids::MSG_CHECK_ROW {
                    self.start_scan(Expect::SignedMessage);
                } else if id == ids::TOOLS_EXPLORER {
                    self.push(Screen::Explore);
                } else if id == ids::TOOLS_WORD_LIST {
                    self.push(Screen::WordList);
                } else if id == ids::TOOLS_DICE {
                    self.push(Screen::DicePassphrase);
                } else if id == ids::TOOLS_DECODE {
                    self.start_scan(Expect::Transaction);
                } else if id == ids::TOOLS_COMPARE {
                    self.compare_tx.clear();
                    self.start_scan(Expect::CompareTransaction);
                } else if id == ids::TOOLS_NOTES {
                    self.push(Screen::Notes);
                } else if id == ids::TOOLS_LIGHTNING {
                    self.overlay = Some(Overlay::Choice(Picker::LightningFrom, 0));
                } else if let Some(i) = ids::index_in(id, ids::TOOLS_CALC_BASE, Tool::ALL.len()) {
                    // Every calculator that takes a string opens on the
                    // scanner, whose "Type" row is the field. Units
                    // takes a number, which no camera reads.
                    match expect_of(Tool::ALL[i]) {
                        Some(expect) => self.start_scan(expect),
                        None => self.push(Screen::Tool(Tool::ALL[i])),
                    }
                }
            }
            Screen::Lightning => self.tap_lightning(id),
            Screen::Vanity(key) => self.tap_vanity(id, key),
            Screen::Tool(tool) => self.tap_tool(id, tool),
            Screen::ToolResult(tool) => self.tap_tool_result(id, tool),
            Screen::Decode => self.tap_decode(id),
            Screen::CompareTx => {
                if id == ids::COMPARE_TX_SECOND {
                    self.start_scan(Expect::CompareTransaction);
                } else if id == ids::COMPARE_TX_DONE {
                    self.go_home();
                }
            }
            Screen::WordList => self.tap_word_list(id),
            Screen::Word(i) => self.tap_word(id, i),
            Screen::DicePassphrase => self.tap_dice(id),
            Screen::WalletName(_) => {}
            Screen::Wallet(wallet) => {
                if id == ids::WALLET_NAME {
                    self.name_entry = String::from(self.wallet_name(wallet).unwrap_or_default());
                    self.push(Screen::WalletName(wallet));
                } else if id == ids::WALLET_SIGN && self.wallet_can_sign(wallet) {
                    self.start_scan(Expect::Psbt);
                } else if id == ids::WALLET_SIGN_MESSAGE
                    && self.wallet_signs_messages(wallet)
                    && self.wallet_can_sign(wallet)
                {
                    self.start_scan(Expect::Message);
                } else if id == ids::SILENT_ADDRESS
                    && let WalletRef::Policy(w) = wallet
                {
                    self.silent_form = silent::AddressForm::Address;
                    self.push(Screen::SilentAddress(w, None));
                } else if id == ids::SILENT_LABELS
                    && let WalletRef::Policy(w) = wallet
                {
                    self.push(Screen::SilentLabels(w));
                } else if id == ids::SILENT_CHECK
                    && let WalletRef::Policy(w) = wallet
                {
                    self.check_payment = None;
                    self.push(Screen::SilentCheck(w));
                } else if id == ids::WALLET_CHECK {
                    self.start_scan(Expect::Address);
                } else if id == ids::WALLET_ADDRESSES {
                    self.push(Screen::Addresses(wallet));
                } else if id == ids::WALLET_EXPORT {
                    self.push(Screen::Export(wallet));
                } else if id == ids::WALLET_SHEET {
                    self.push(Screen::Sheet(wallet));
                } else if id == ids::WALLET_KEYS {
                    if let WalletRef::Policy(w) = wallet {
                        self.push(Screen::WalletKeys(w));
                    }
                } else if id == ids::WALLET_KEEP
                    && let WalletRef::Policy(policy) = wallet
                    && self.wallet_keep_offered()
                {
                    // §4.2 Toggle: the row is the choice, so the tap
                    // applies it and the page stays.
                    let kept = self.wallet_is_kept(policy);
                    self.set_wallet_kept(policy, !kept);
                } else if id == ids::WALLET_FORGET
                    && let WalletRef::Policy(policy) = wallet
                    && policy < self.wallets.len()
                {
                    // A wallet is public data, so dropping it is a tap:
                    // what it removes is a row, not a key.
                    let removed = self.wallets.remove(policy);
                    self.wallets_not_kept.retain(|c| *c != removed.checksum());
                    self.sync_kept();
                    self.back();
                }
            }
            Screen::Addresses(owner) => {
                if id == ids::ADDR_SCRIPT && matches!(owner, WalletRef::Key(_)) {
                    self.open_script_choice(self.detail.script);
                } else if id == ids::ADDR_RECEIVE || id == ids::ADDR_CHANGE {
                    self.detail = DetailState {
                        script: self.detail.script,
                        change: id == ids::ADDR_CHANGE,
                        ..DetailState::default()
                    };
                } else if id == ids::ADDR_MORE {
                    self.detail.shown =
                        (self.detail.shown + ADDRESS_PAGE).min(MAX_ADDRESS_INDEX as usize + 1);
                } else if let Some(i) = ids::index_in(id, ids::ADDR_ROW_BASE, self.detail.shown) {
                    self.detail.open = Some(i as u32);
                }
            }
            Screen::BackupMenu(key) => {
                if id == ids::BACKUP_VERIFY {
                    self.start_backup(key, BackupStep::QuizStart);
                } else if id == ids::BACKUP_WORDS {
                    self.start_backup(key, BackupStep::Words);
                } else if id == ids::BACKUP_SEEDQR {
                    self.start_backup(key, BackupStep::SeedQr);
                } else if id == ids::BACKUP_COMPACT {
                    self.start_backup(key, BackupStep::CompactSeedQr);
                } else if id == ids::BACKUP_ENCRYPTED {
                    self.ask_form(FormFor::Backup(key));
                } else if id == ids::BACKUP_GRID {
                    self.start_backup(key, BackupStep::GridChoice);
                } else if id == ids::BACKUP_STEEL {
                    self.start_backup(key, BackupStep::Steel);
                } else if id == ids::BACKUP_SLIP39 {
                    self.start_share_backup(key);
                } else if id == ids::BACKUP_CODEX32 {
                    self.start_codex32_backup(key);
                } else if id == ids::BACKUP_XOR {
                    self.start_backup(key, BackupStep::XorCount);
                }
            }
            Screen::OpenChild(_) => self.tap_open_child(id),
            Screen::Bip85(_) => {
                if id == ids::BIP85_DONE {
                    self.back();
                }
            }
            Screen::OpenPassphrase(_) => {}
            // The key is added already; the action opens its own menu,
            // and the chevron from there goes to the key list, as it
            // does from this screen (§16.67).
            Screen::Opened(key, _) => {
                if id == ids::OPENED_OPEN && self.keys.get(key).is_some() {
                    // The way on is the key's own page, where its facts
                    // and its backup are.
                    self.screen = Screen::KeyDetail(key);
                    self.entered();
                }
            }
            Screen::Learn => {
                if let Some(i) = ids::index_in(id, ids::LEARN_ROW_BASE, osk_learn::PAGES) {
                    self.push(Screen::LearnPage(i));
                }
            }
            Screen::KeptRemoved => {
                if id == ids::KEEP_REMOVED_DONE {
                    self.go_home();
                }
            }
            Screen::Keep(_) => {
                // "Not now" on the offer does what the chevron does:
                // the key stays added and nothing is kept (§16.65).
                if id == ids::KEEP_NOT_NOW {
                    self.back();
                }
            }
            Screen::DuressPin
            | Screen::Forget(_)
            | Screen::Tiers
            | Screen::WipeAll
            | Screen::WipeAndExit => {}
            Screen::LearnPage(page) => {
                if id == ids::LEARN_TRY {
                    self.open_learn_try(page);
                }
            }
            // A page opened from the info button has no row: the
            // chevron is the whole of it.
            Screen::LearnTopic(..) => {}
            Screen::Wiped(_) => {
                if id == ids::SETTINGS_WIPED_DONE {
                    self.go_home();
                }
            }
            // The start screen's one control is handled above.
            Screen::Ended | Screen::NoSecureBoot => {}
            // §16.110 rule 3: a loaded row opens the key's page, and a
            // row this device has no key for opens Add a key, which
            // comes back to this review with the transaction inspected
            // again.
            Screen::SignKeys(reading) => {
                let members = self.transaction_keys(reading);
                if let Some(i) = ids::index_in(id, ids::SIGN_KEY_ROW_BASE, members.len()) {
                    match members[i].2 {
                        Some(k) => self.push(Screen::KeyDetail(k)),
                        None => {
                            let flow = match reading {
                                true => core::mem::take(&mut self.decode),
                                false => core::mem::take(&mut self.sign),
                            };
                            self.key_detour = Some(KeyDetour { flow, reading });
                            self.add_key_return = Some(AddKeyReturn {
                                to: ReturnTo::Transaction,
                                member: members[i].0,
                            });
                            self.push(Screen::Add);
                        }
                    }
                }
            }
            Screen::KeyExport(key, account, step) => self.tap_key_export(id, key, account, step),
            Screen::Export(owner) => {
                let owner = Exported::Wallet(owner);
                if id == ids::EXPORT_SCRIPT && matches!(owner, Exported::Wallet(WalletRef::Key(_)))
                {
                    self.open_script_choice(self.export.script);
                } else if id == ids::EXPORT_FORMAT {
                    self.overlay = Some(Overlay::Choice(Picker::Format, self.export.format));
                } else if id == ids::EXPORT_SHOW {
                    self.show_export_qr(owner);
                } else if id == ids::EXPORT_ANIMATED {
                    // A payload one code cannot carry at the floor stays
                    // on the parts; the row is dimmed there anyway (§4.9).
                    let next = !self.export.animated || !self.export.scans;
                    self.set_export_qr_mode(owner, next);
                } else if id == ids::EXPORT_TEXT {
                    self.open_export_compare(owner);
                } else if id == ids::EXPORT_SAVE {
                    self.save_core_import(owner);
                } else if id == ids::SILENT_SECRET_ROW
                    && let Exported::Wallet(WalletRef::Policy(wallet)) = owner
                {
                    self.push(Screen::SilentSecret(wallet));
                }
            }
            Screen::Verify => {
                // "Check another" goes back to the scanner, which is
                // where an address comes from.
                if id == ids::VERIFY_CLEAR {
                    self.pop_screen();
                    self.start_scan(Expect::Address);
                } else if id == ids::VERIFY_ADDRESS {
                    let address = String::from(self.verify.input());
                    self.open_compare(self.strings().row_address, address);
                }
            }
            Screen::Scan => self.tap_scan(id),
            Screen::Inspect => {
                if id == ids::INSPECT_DONE {
                    self.back();
                } else if id == ids::INSPECT_USE_WALLET {
                    self.use_reviewed_wallet();
                } else if id == ids::INSPECT_FORGET_WALLET {
                    if let Some(policy) = self.inspect.as_ref().and_then(|d| d.policy.clone()) {
                        self.wallets.retain(|w| *w != policy);
                        self.sync_kept();
                    }
                    self.ui.set_scroll(ids::SCROLL, 0);
                } else if id == ids::INSPECT_TEXT
                    && let Some(d) = self.inspect.as_ref()
                {
                    let (label, value, descriptor) = (d.title, d.text.clone(), d.descriptor);
                    if descriptor {
                        self.open_compare_descriptor(label, value);
                    } else {
                        self.open_compare(label, value);
                    }
                }
            }
            Screen::Settings => {
                if id == ids::SETTINGS_NETWORK_ROW {
                    self.overlay = Some(Overlay::Setting(Setting::Network));
                } else if id == ids::SETTINGS_UNIT_ROW {
                    self.overlay = Some(Overlay::Setting(Setting::Unit));
                } else if id == ids::SETTINGS_CAMERA_ROTATION_ROW && !self.camera_fixed {
                    self.overlay = Some(Overlay::Setting(Setting::CameraRotation));
                } else if id == ids::SETTINGS_NONCE_ROW {
                    self.overlay = Some(Overlay::Setting(Setting::Nonce));
                } else if id == ids::SETTINGS_SCHNORR_ROW {
                    self.overlay = Some(Overlay::Setting(Setting::Schnorr));
                } else if id == ids::SETTINGS_BACKUP_MEMORY_ROW {
                    self.overlay = Some(Overlay::Setting(Setting::BackupMemory));
                } else if id == ids::SETTINGS_LOCK_AFTER_ROW {
                    self.overlay = Some(Overlay::Setting(Setting::Lock));
                } else if id == ids::SETTINGS_WIPE_AFTER_ROW {
                    self.overlay = Some(Overlay::Setting(Setting::Wipe));
                } else if id == ids::SETTINGS_ABOUT_ROW {
                    self.push(Screen::About);
                } else if id == ids::SETTINGS_START_HERE_ROW {
                    self.push(Screen::StartHere);
                } else if id == ids::SETTINGS_WIPE_ROW {
                    self.push(Screen::WipeAll);
                } else if id == ids::SETTINGS_EXIT_ROW {
                    self.push(Screen::WipeAndExit);
                } else if id == ids::SETTINGS_LOCK {
                    self.lock();
                } else if id == ids::KEEP_DURESS_ROW && self.kept_rows_offered() {
                    self.keep.clear();
                    self.push(Screen::DuressPin);
                } else if id == ids::SETTINGS_SCRAMBLE {
                    let on = !self.session.scramble_pin();
                    self.session.set_scramble_pin(on);
                    self.store_settings();
                }
            }
            Screen::About => {
                if id == ids::SETTINGS_SELFTEST {
                    self.run_selftest();
                } else if id == ids::ABOUT_TIER {
                    self.push(Screen::Tiers);
                } else if id == ids::ABOUT_HASH
                    && let Some(hash) = self.build.core_hash
                {
                    let label = self.strings().settings_core_hash;
                    self.open_compare(label, hash);
                }
            }
            Screen::Sign => self.tap_sign(id),
            Screen::SignMessage => self.tap_sign_message(id),
            Screen::CheckedMessage => self.tap_checked_message(id),
        }
    }

    /// One step back inside Explore, which [`Explore::back`] decides:
    /// a screen returns to the menu whose row opened it, and the key
    /// menu asks before it drops typed words.
    fn explore_back(&mut self) -> bool {
        self.explore.back()
    }

    /// The review step after `step`, or the confirm step's own actions.
    fn sign_next(&mut self, step: sign::Step) {
        if let Some(next) = self.sign.after(step) {
            self.sign.go(next);
        }
    }

    /// Opens band tile `t`.
    /// The wallet a Learn page's row means by "the first wallet": the
    /// first loaded key's own, or the first policy in use.
    fn first_wallet(&self) -> Option<WalletRef> {
        (!self.wallets.is_empty()).then_some(WalletRef::Policy(0))
    }

    /// The last row of a Learn page: the flow the page is about. A row
    /// whose flow needs a key that is not loaded opens Add instead.
    fn open_learn_try(&mut self, page: usize) {
        let Some((what, _, _)) = self.learn_try(page) else {
            return;
        };
        let has_key = !self.keys.is_empty();
        match what {
            TryIt::CreateKey => self.start_create(),
            TryIt::RollDice => {
                self.start_create();
                if let Some(Wizard::Create(w)) = &mut self.wizard {
                    w.go(create::Step::Count);
                }
            }
            TryIt::NewWallet => self.start_build(),
            TryIt::Miniscript => self.push(Screen::Tool(Tool::Miniscript)),
            TryIt::VerifyBackup if has_key => self.push(Screen::BackupMenu(0)),
            TryIt::EncryptedBackup if has_key => self.ask_form(FormFor::Backup(0)),
            TryIt::OpenPassphrase if self.keys.first().is_some_and(LoadedKey::has_mnemonic) => {
                self.push(Screen::OpenPassphrase(0));
                self.open_finish = Some(Finish::new());
            }
            TryIt::SignMessage if has_key => self.start_scan(Expect::Message),
            TryIt::CheckAddress => match self.first_wallet() {
                Some(_) => self.start_scan(Expect::Address),
                None => self.start_scan(Expect::Any),
            },
            // An export is a wallet's; with none registered the row
            // opens the menu that makes one.
            TryIt::Export => match self.first_wallet() {
                Some(wallet) => self.push(Screen::Export(wallet)),
                None => self.push(Screen::AddWallet),
            },
            _ => self.push(Screen::Add),
        }
    }

    fn open_tile(&mut self, t: usize) {
        match t {
            TILE_WALLETS => self.push(Screen::Wallets),
            TILE_KEYS => self.push(Screen::Keys),
            // Scan is the one way in for anything read, and it routes by
            // what it reads (§16.104 rule 4).
            TILE_SCAN => self.start_scan(Expect::Any),
            TILE_TOOLS => self.push(Screen::Tools),
            TILE_LEARN => self.push(Screen::Learn),
            _ => self.push(Screen::Settings),
        }
    }

    fn tap_explore(&mut self, id: Id) {
        let network = self.network;
        let keys = self.keys.len();
        if id == ids::EXPLORE_TYPE {
            self.start_scan(Expect::Seed);
            return;
        }
        if id == ids::EXPLORE_LOAD {
            self.go_home();
            self.start_load();
            return;
        }
        if id == ids::EXPLORE_DISCARD {
            // The words go, and with them the reason to stay.
            self.explore.zeroize();
            self.explore.go(explore::Step::Keys);
            self.back();
            return;
        }
        if id == ids::EXPLORE_ADDRESSES {
            match self.explore.source() {
                Source::Loaded(k) => self.push(Screen::Addresses(WalletRef::Key(k))),
                // Typed words are a key: the list is derived from them
                // and the passphrase the screen holds.
                Source::Typed => self.push(Screen::Addresses(WalletRef::Typed)),
                Source::None => {}
            }
            return;
        }
        // §4.5: every reference row opens the whole string on Compare.
        if let Some((label, value)) = self.explore_reference(id) {
            self.open_compare(label, value);
            return;
        }
        let words = self
            .explore_with_mnemonic(osk_bip::bip39::Mnemonic::word_count)
            .unwrap_or(0);
        let lang = self
            .explore_with_mnemonic(osk_bip::bip39::Mnemonic::language)
            .unwrap_or(osk_bip::bip39::Language::English);
        let per_page = views::words::per_page(self.class(), self.words_pane_dp(), lang);
        let pages = words.div_ceil(per_page).max(1) as u8;
        // A tap that opens another screen starts it at the top; a tap
        // inside one — a check, a preset, a page — keeps its place.
        let mut restart = true;
        let e = &mut self.explore;
        if id == ids::EXPLORE_KEEP {
            e.go(explore::Step::Keys);
        } else if id == ids::EXPLORE_USING {
            e.go(explore::Step::Chooser);
        } else if id == ids::EXPLORE_TYPED {
            restart = false;
            e.pick(Source::Typed);
        } else if let Some(i) = ids::index_in(id, ids::EXPLORE_KEY_BASE, keys) {
            restart = false;
            e.pick(Source::Loaded(i));
        } else if id == ids::EXPLORE_CHOOSE_CONTINUE {
            e.confirm_pick();
        } else if id == ids::EXPLORE_BITS {
            e.go(explore::Step::Bits);
        } else if id == ids::EXPLORE_WORDS {
            e.go(explore::Step::Words);
        } else if id == ids::EXPLORE_WORDS_DONE {
            e.go(explore::Step::Bits);
        } else if id == ids::WORDS_NUMBERS {
            restart = false;
            e.toggle_numbers();
        } else if id == ids::WORDS_PREV {
            restart = false;
            e.set_word_page(e.word_page().saturating_sub(1));
        } else if id == ids::WORDS_NEXT {
            restart = false;
            e.set_word_page((e.word_page() + 1).min(pages.saturating_sub(1)));
        } else if let Some(i) = ids::index_in(id, ids::EXPLORE_PRESET_BASE, ScriptType::ALL.len()) {
            restart = false;
            e.preset(ScriptType::ALL[i], network);
        } else if id == ids::EXPLORE_RECEIVE {
            restart = false;
            e.set_chain(false);
        } else if id == ids::EXPLORE_CHANGE {
            restart = false;
            e.set_chain(true);
        } else if id == ids::EXPLORE_PATH {
            e.go(explore::Step::Path);
            // §4.6: "On `small` the groups scroll from the top and the
            // checked row is scrolled into view when the editor opens."
            self.scroll_into_view = self.checked_preset().map(|id| (id, Anchor::InView));
        } else if id == ids::EXPLORE_PASSPHRASE {
            e.go(explore::Step::Passphrase);
            self.mask_deadline = e.mask_deadline();
        } else if let Some(secret) = explore_secret_of(id) {
            e.go(explore::Step::Secret(secret));
        } else {
            restart = false;
        }
        if restart {
            self.ui.set_scroll(ids::SCROLL, 0);
        }
    }

    /// The id of the purpose row the path editor's check is on, for the
    /// scroll §4.6 asks for when the editor opens.
    fn checked_preset(&self) -> Option<Id> {
        let network = self.network;
        ScriptType::ALL
            .iter()
            .position(|sc| self.explore.at_preset(*sc, network))
            .map(|i| ids::at(ids::EXPLORE_PRESET_BASE, i))
    }

    /// The label and the whole string a reference row of Explore's key
    /// menu opens on the Compare screen (§4.5).
    fn explore_reference(&self, id: Id) -> Option<(&'static str, String)> {
        let s = self.strings();
        let cached = self.explore_cached()?;
        match id {
            ids::EXPLORE_ACCOUNT_XPUB => {
                Some((s.explore_account_xpub, cached.account_xpub.clone()))
            }
            ids::EXPLORE_SLIP132 => cached
                .account_slip132
                .as_ref()
                .map(|(_, value)| (s.explore_slip132, value.clone())),
            ids::EXPLORE_XPUB => Some((
                s.explore_level_xpub,
                match cached.levels.last() {
                    Some(level) => level.xpub.clone(),
                    None => cached.master_xpub.clone(),
                },
            )),
            _ => None,
        }
    }

    fn tap_scan(&mut self, id: Id) {
        let Some(s) = self.scan.as_mut() else {
            return;
        };
        // "Try again" is the way off the Result a code that could not be
        // used takes over, whatever stage the scanner was in.
        if s.error().is_some() {
            if id == ids::SCAN_AGAIN {
                s.resume();
                self.ui.set_scroll(ids::SCROLL, 0);
            }
            return;
        }
        // §4.9 puts the three rows under the viewfinder wherever the
        // camera is: live, absent, or waiting on the last request.
        let under_viewfinder = matches!(
            s.stage(),
            ScanStage::Camera | ScanStage::Unavailable | ScanStage::Waiting
        );
        if id == ids::SCAN_FILE && under_viewfinder {
            s.request_file();
            self.commands.push_back(Command::RequestFile {
                kind: FileKind::Any,
            });
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if id == ids::SCAN_PASTE && under_viewfinder && self.has_clipboard {
            self.commands.push_back(Command::RequestClipboard {
                kind: FileKind::Text,
            });
            return;
        }
        if id == ids::SCAN_TYPE && under_viewfinder {
            let expect = s.expect();
            self.scan_type(expect);
            return;
        }
        if id == ids::SCAN_USE_KEY && under_viewfinder && s.expect() == Expect::ConvertKey {
            self.pop_screen();
            self.push(Screen::Tool(Tool::ConvertKey));
            self.open_convert_key_choice();
            return;
        }
        match s.stage() {
            ScanStage::Camera | ScanStage::Waiting | ScanStage::Unavailable => {}
            // The passphrase entry is the keyboard's; nothing else on
            // that screen is tapped.
            ScanStage::BackupPassphrase => {}
            ScanStage::Unknown => {
                if id == ids::SCAN_HEX {
                    let hex = text::hex(s.unknown());
                    let label = self.strings().row_hex;
                    self.open_compare(label, hex);
                } else if id == ids::SCAN_AS_PSBT {
                    let bytes = s.take_unknown();
                    self.sign_scanned(&bytes);
                } else if id == ids::SCAN_AS_HASHES {
                    // The bytes as they were read, hashed: the mode is
                    // hex because that is what the field now holds.
                    let bytes = s.take_unknown();
                    let hex = text::hex(&bytes);
                    self.tool_scanned_as(
                        Tool::Hashes,
                        hex,
                        Some(osk_codec::encodings::ReadAs::Hex),
                    );
                } else if id == ids::SCAN_AS_ENCODINGS {
                    let bytes = s.take_unknown();
                    let text = String::from_utf8_lossy(bytes.trim_ascii()).into_owned();
                    self.tool_scanned(Tool::Encodings, text);
                } else if id == ids::SCAN_AS_TEXT {
                    let bytes = s.take_unknown();
                    let title = self.strings().inspect_text;
                    self.inspect_scanned(InspectDoc {
                        title,
                        text: String::from_utf8_lossy(&bytes).into_owned(),
                        descriptor: false,
                        checksum: None,
                        policy: None,
                        name: None,
                        from_menu: false,
                        swap: None,
                        bsms: None,
                    });
                }
            }
            ScanStage::WordsCaution => {
                if id == ids::SCAN_CONTINUE
                    && let Some(w) = s.take_pending()
                {
                    self.words_to_wizard(w);
                }
            }
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    fn tap_sign(&mut self, id: Id) {
        match self.sign.stage() {
            // A transaction reaches this screen by arriving, so there
            // is no entry here to answer; the error a bad one leaves is
            // read and left.
            Stage::Entry | Stage::Unavailable | Stage::Waiting => {}
            Stage::ParseError => {
                if id == ids::SIGN_BACK {
                    self.back();
                }
            }
            Stage::Wizard(step) => {
                // §16.110 rule 3: the key context row opens the
                // transaction's own Keys review.
                if id == ids::SIGN_KEYS {
                    self.open_transaction_keys(false);
                    return;
                }
                let insp_keys = self
                    .sign
                    .inspection()
                    .map_or(0, |i| i.participating_keys.len());
                match step {
                    sign::Step::Summary => {
                        if id == ids::SIGN_TO {
                            self.compare_recipient();
                            return;
                        } else if id == ids::SIGN_CONTINUE && self.sign.participates() {
                            self.sign_next(step);
                        }
                    }
                    sign::Step::Outputs => {
                        if id == ids::SIGN_CONTINUE {
                            if self.sign.next_output() {
                                return;
                            }
                            self.sign_next(step);
                        } else if self.compare_output(id) {
                            return;
                        }
                    }
                    sign::Step::Inputs => {
                        if id == ids::SIGN_CONTINUE {
                            self.sign_next(step);
                        } else if id == ids::SIGN_SIGNATURES {
                            self.sign.open_signatures(step);
                        }
                    }
                    sign::Step::Warnings => {
                        if id == ids::SIGN_ACK {
                            // A blocked transaction is offered no
                            // acknowledgement row, and answers none.
                            if !self.sign.has_blocked() {
                                self.sign.toggle_acknowledged();
                            }
                            return;
                        } else if id == ids::SIGN_CONTINUE && self.sign.can_acknowledge() {
                            self.sign_next(step);
                        }
                    }
                    sign::Step::Confirm => {
                        if id == ids::SIGN_TO {
                            self.compare_recipient();
                        } else if let Some(i) = ids::index_in(id, ids::SIGN_KEY_BASE, insp_keys) {
                            self.sign.toggle_key(i);
                        } else if let Some(i) =
                            ids::index_in(id, ids::SIGN_OTHER_BASE, THRESHOLD_MAX_SHARES)
                        {
                            // The "Then with" row: the shares that will
                            // sign at a later location (§16.103).
                            let id = i as u32;
                            if self.sign.threshold_candidates().contains(&id) {
                                self.sign.toggle_other(id);
                            }
                        }
                        return;
                    }
                    sign::Step::Result => {
                        if id == ids::SIGN_SAVE {
                            if let Some(out) = self
                                .sign
                                .outcome()
                                .filter(|o| o.save != sign::Save::Waiting)
                            {
                                self.commands.push_back(Command::WriteFile {
                                    kind: if out.complete || out.carry.is_some() {
                                        FileKind::Any
                                    } else {
                                        FileKind::Psbt
                                    },
                                    name_hint: out.name_hint.clone(),
                                    bytes: out.bytes.clone(),
                                });
                                self.sign.mark_saving();
                            }
                            return;
                        } else if id == ids::SIGN_SIGNATURES {
                            self.sign.open_signatures(sign::Step::Result);
                        } else if id == ids::SIGN_QR {
                            self.sign.show_qr(self.now_ms, self.qr_side_px(), self.dpi);
                        } else if id == ids::SIGN_DONE {
                            self.go_home();
                            return;
                        }
                    }
                    sign::Step::Signatures => {
                        if self.compare_signature(id, false) {
                            return;
                        }
                    }
                    sign::Step::Qr => {
                        if id == ids::SIGN_QR_ANIMATED {
                            // A payload too long for one code stays on
                            // the animated parts.
                            let next = if self.sign.qr_mode() == Some(QrMode::Ur)
                                && self.sign.static_qr_fits()
                            {
                                QrMode::Static
                            } else {
                                QrMode::Ur
                            };
                            self.sign.set_qr_mode(next, self.now_ms);
                        }
                        return;
                    }
                }
            }
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The recipient's address, whole, on the Compare screen (§4.5).
    fn compare_recipient(&mut self) {
        let address = self.sign.inspection().and_then(|i| {
            i.outputs
                .iter()
                .position(|o| !o.is_ours())
                .map(|n| (n, i.outputs[n].address.clone()))
        });
        if let Some((index, address)) = address {
            self.compare_address(index, address);
        }
    }

    /// §5 Compare: an output's address, with the output's name over it.
    fn compare_address(&mut self, index: usize, address: String) {
        let s = self.strings();
        let (title, name) = (s.row_address, s.sign_output_name);
        let label = strings::fill1(name, &alloc::format!("{}", index + 1));
        self.open_compare_one(title, label, address);
    }

    /// The output on screen, whose address is a reference row.
    fn compare_output(&mut self, id: Id) -> bool {
        let output = self.sign.output();
        let address = self
            .sign
            .inspection()
            .and_then(|i| i.outputs.get(output))
            .filter(|o| ids::at(ids::SIGN_OUT_BASE, o.index) == id)
            .map(|o| o.address.clone());
        match address {
            Some(address) => {
                self.compare_address(output, address);
                true
            }
            None => false,
        }
    }

    /// The transaction id and each signature, whole, from the Signatures
    /// page (§4.5). `reading` picks the flow the page belongs to, so
    /// that Decode's own page opens its signatures the same way.
    fn compare_signature(&mut self, id: Id, reading: bool) -> bool {
        let flow = if reading { &self.decode } else { &self.sign };
        if id == ids::SIGN_TXID {
            let Some(txid) = flow.outcome().and_then(|o| o.txid) else {
                return false;
            };
            let value = alloc::format!("{txid}");
            let label = self.strings().sign_txid;
            self.open_compare(label, value);
            return true;
        }
        let rows = flow.signature_rows();
        let Some(i) = ids::index_in(id, ids::SIGN_SIG_BASE, rows.len()) else {
            return false;
        };
        let value = text::hex(&rows[i].bytes);
        // §5 Compare: the title is the kind and the label the instance,
        // which for a signature is the input it signs and the key that
        // signed it.
        let label = alloc::format!("#{} \u{00b7} {}", rows[i].input, rows[i].name);
        let title = self.strings().sign_signature;
        self.open_compare_one(title, label, value);
        true
    }

    /// The shell answered a file request: the Sign entry's, or the
    /// scanner's file fallback, whose content is routed like a code.
    /// The answer to a `WriteFile`: the file row stops waiting and says
    /// what happened. Nothing else on the screen moves, and an answer
    /// for a save nobody is waiting on is ignored.
    fn write_answer(&mut self, written: bool) {
        if self.png_save == sign::Save::Waiting {
            self.png_save = Save::Idle;
            let s = self.strings();
            let line = if written {
                strings::fill1(s.png_saved, s.png_file_name)
            } else {
                String::from(s.png_not_saved)
            };
            self.say(&line);
            self.render();
            return;
        }
        if self.export_save == sign::Save::Waiting {
            self.export_save = if written {
                sign::Save::Written
            } else {
                sign::Save::Failed
            };
            self.render();
            return;
        }
        if self.sealed_save == sign::Save::Waiting {
            self.sealed_save = if written {
                sign::Save::Written
            } else {
                sign::Save::Failed
            };
            self.render();
            return;
        }
        if let Some(Wizard::Backup(b)) = &mut self.wizard
            && b.save() == sign::Save::Waiting
        {
            b.mark_saved(written);
            self.render();
            return;
        }
        if let Some(Wizard::Build(w)) = &mut self.wizard
            && let Some(d) = w.dealer_mut()
            && d.save() == sign::Save::Waiting
        {
            d.mark_saved(written);
            self.render();
            return;
        }
        if let Some(flow) = self.message.as_mut().filter(|f| f.saving()) {
            flow.mark_saved(written);
            self.render();
            return;
        }
        if self.sign.outcome().map(|o| o.save) != Some(sign::Save::Waiting) {
            return;
        }
        if written {
            self.sign.mark_saved();
        } else {
            self.sign.mark_not_saved();
        }
        self.render();
    }

    /// The answer to a file request from a shell that lists what it
    /// holds: the files go on the Files screen over whatever asked, and
    /// the tap on one of them reads it (`docs/DESIGN.md` §5).
    fn file_list_event(&mut self, kind: FileKind, entries: Vec<FileEntry>, place: Option<String>) {
        if self.wizard.is_some() || !self.waiting_for_file() {
            return;
        }
        self.overlay = Some(Overlay::Files(FileListing {
            kind,
            entries,
            place,
        }));
        self.ui.set_scroll(ids::SCROLL, 0);
        self.render();
    }

    /// Whether a screen has a file request out.
    fn waiting_for_file(&self) -> bool {
        (self.screen == Screen::Sign && self.sign.stage() == Stage::Waiting)
            || self.scan_stage() == Some(ScanStage::Waiting)
    }

    /// The request the file list answered is over and no file was
    /// chosen: the Sign entry rows come back and the scanner goes back
    /// to scanning, as they do when a shell has no file to give.
    fn cancel_file_request(&mut self) {
        if self.screen == Screen::Sign && self.sign.stage() == Stage::Waiting {
            self.sign.clear();
        } else if self.scan_stage() == Some(ScanStage::Waiting)
            && let Some(s) = self.scan.as_mut()
        {
            s.file_unavailable();
        }
    }

    /// The shell's picker closed with nothing chosen. The person did
    /// that on purpose and knows it, so the screen they were on comes
    /// back as they left it and says nothing about it (`docs/UX.md` F4).
    fn file_cancelled(&mut self) {
        if self.wizard.is_some() {
            return;
        }
        if self.screen == Screen::Sign && self.sign.stage() == Stage::Waiting {
            self.sign.cancelled();
        } else if self.scan_stage() == Some(ScanStage::Waiting) {
            if let Some(s) = self.scan.as_mut() {
                s.file_cancelled();
            }
        } else {
            return;
        }
        self.ui.set_scroll(ids::SCROLL, 0);
        self.render();
    }

    fn file_event(&mut self, bytes: Option<Vec<u8>>) {
        // A wizard reads no file, except the builder, whose scanner
        // takes one the way every other scanner does.
        if self.wizard.is_some() && !self.building_scan() {
            return;
        }
        if self.screen == Screen::Sign && self.sign.stage() == Stage::Waiting {
            match bytes {
                Some(bytes) => {
                    let refs = self.key_refs();
                    let shares = self.share_refs();
                    let network = self.network;
                    let wallets = self.wallets.clone();
                    let view = self.musig_session.as_ref().map(|s| s.view());
                    self.sign
                        .load(&bytes, refs, wallets, shares, network, view.as_ref());
                    self.check_transaction_signatures(false);
                }
                None => self.sign.unavailable(),
            }
        } else if self.scan_stage() == Some(ScanStage::Waiting) {
            match bytes {
                Some(bytes) => {
                    if let Some(s) = self.scan.as_mut() {
                        s.resume();
                    }
                    self.route_payload(Zeroizing::new(bytes), true);
                }
                None => {
                    if let Some(s) = self.scan.as_mut() {
                        s.file_unavailable();
                    }
                }
            }
        } else {
            return;
        }
        self.ui.set_scroll(ids::SCROLL, 0);
        self.render();
    }

    /// The answer to a `RequestClipboard`: the text goes through the
    /// router exactly as a scanned or read payload does, and nothing at
    /// all leaves the viewfinder saying so for a moment.
    fn clipboard_event(&mut self, text: Option<String>) {
        if self.scan.is_none() || self.screen != Screen::Scan {
            return;
        }
        match text {
            Some(text) => {
                if let Some(s) = self.scan.as_mut() {
                    s.resume();
                    s.set_pasted();
                }
                self.route_payload(Zeroizing::new(text.into_bytes()), true);
            }
            None => {
                let line = String::from(self.strings().scan_nothing_to_paste);
                let until = self.now_ms + NOTICE_MS;
                if let Some(s) = self.scan.as_mut() {
                    s.set_note(line, until);
                }
            }
        }
        self.ui.set_scroll(ids::SCROLL, 0);
        self.render();
    }

    /// The answer to a `WriteClipboard`: the screen says what happened
    /// on its caption line, and a shell that could not do it has said,
    /// once and for the session, that it has no clipboard.
    fn copy_answer(&mut self, written: bool) {
        let s = self.strings();
        let line = if written { s.copy_done } else { s.copy_none };
        if !written {
            self.has_clipboard = false;
        }
        self.say(line);
        self.render();
    }

    fn tap_wizard(&mut self, id: Id) {
        let network = self.network;
        let now = self.now_ms;
        let class = self.class();
        // The plan's own screens, and the gatherer's inside a Backup's
        // split, before anything the flow underneath answers for.
        if self.on_share_plan() {
            self.tap_shares(id);
            return;
        }
        if self.on_codex32_plan() {
            self.tap_codex32(id);
            return;
        }
        if matches!(&self.wizard, Some(Wizard::Backup(b)) if b.step() == BackupStep::Shares) {
            self.tap_share_gather(id, network);
            return;
        }
        if matches!(&self.wizard, Some(Wizard::Backup(b)) if b.step() == BackupStep::Codex32) {
            self.tap_codex32_gather(id, network);
            return;
        }
        // The source step is a choice: the tap checks a row, Continue
        // acts on it, and the scan row's action is the scanner.
        if id == ids::LOAD_SOURCE_CONTINUE
            && self.has_camera
            && let Some(Wizard::Load(w)) = &self.wizard
            && w.step() == Step::Source
        {
            let expect = match w.source() {
                load::Source::SeedCode => Some(Expect::Seed),
                load::Source::Backup => Some(Expect::Backup),
                load::Source::Type
                | load::Source::SeedXor
                | load::Source::Slip39
                | load::Source::Codex32
                | load::Source::Numbers
                | load::Source::Hex => None,
            };
            if let Some(expect) = expect {
                self.cancel_wizard();
                self.start_scan(expect);
                return;
            }
            // Combining Seed XOR parts takes a word count and a
            // wordlist and then one part after another, which is the
            // Create wizard's own run of steps.
            if w.source() == load::Source::SeedXor {
                self.wizard = Some(Wizard::Create(CreateWizard::combining()));
                self.ui.set_scroll(ids::SCROLL, 0);
                self.render();
                return;
            }
        }
        // Explore's word entry ends at a valid checksum: the indices go to
        // the workspace and the wizard is dropped without adding a key.
        if self.explore_entry
            && id == ids::LOAD_CONTINUE
            && let Some(Wizard::Load(w)) = &self.wizard
            && w.step() == Step::Checksum
            && w.checksum_ok()
        {
            if let Some(m) = w.mnemonic() {
                self.explore.set_typed(m.indices(), m.language(), network);
            }
            self.cancel_wizard();
            return;
        }
        if id == ids::LOAD_HOLD {
            self.confirm_wizard();
            return;
        }
        if (id == ids::BACKUP_SAVE || id == ids::BACKUP_SHOW_QR)
            && matches!(&self.wizard, Some(Wizard::Backup(b))
                if b.step() == BackupStep::Encrypted)
        {
            self.tap_encrypted(id);
            return;
        }
        if id == ids::XOR_COUNT_CONTINUE
            && matches!(&self.wizard, Some(Wizard::Backup(b))
                if b.step() == BackupStep::XorCount)
        {
            if let Some(Wizard::Backup(b)) = &mut self.wizard {
                b.go_to_source();
            }
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        if id == ids::XOR_SOURCE_CONTINUE
            && matches!(&self.wizard, Some(Wizard::Backup(b))
                if b.step() == BackupStep::XorSource)
        {
            self.start_gather(0);
            return;
        }
        // The random parts are gathered in a Create wizard, so its own
        // screens are driven the way Create a key's are.
        if matches!(&self.wizard, Some(Wizard::Backup(b))
            if b.step() == BackupStep::XorGather)
        {
            self.tap_gather(id, network);
            return;
        }
        if id == ids::BACKUP_STEEL_TEMPLATE
            && matches!(&self.wizard, Some(Wizard::Backup(b))
                if b.step() == BackupStep::Steel)
        {
            self.write_steel_template();
            return;
        }
        let pane = self.words_pane_dp();
        let Some(w) = &mut self.wizard else {
            return;
        };
        match w {
            Wizard::Load(w) => w.tap(id, network),
            Wizard::Build(_) => {
                self.tap_build(id);
                return;
            }
            Wizard::Create(w) => {
                // The shutter and the mixed result's reference rows are
                // the app's, not the wizard's: one holds the frame the
                // viewfinder drew, the other opens the Compare screen.
                if id == ids::CREATE_SHUTTER && w.step() == create::Step::Camera {
                    self.create_shutter();
                    self.sync_create();
                    self.render();
                    return;
                }
                if w.step() == create::Step::MixResult
                    && let Some(i) =
                        ids::index_in(id, ids::CREATE_MIX_BASE, osk_entropy::MIX_SOURCES.len())
                    && let Some(c) = w.mix_commitments().get(i).copied()
                {
                    let title = self.strings().create_mixed_title;
                    self.open_compare(title, text::hex(&c));
                    return;
                }
                let per_page = views::words::per_page(class, pane, w.language());
                let before = (w.step(), w.xor_parts());
                w.tap(id, network, now, per_page);
                // A random value of a SLIP-39 split is gathered on this
                // wizard's own screens and taken where its words would
                // be, which it never shows.
                if w.share_gather().is_some() && w.step() == create::Step::Words {
                    if w.is_codex32() {
                        self.codex32_gathered();
                    } else {
                        self.share_gathered();
                    }
                    self.render();
                    return;
                }
                if w.step() == create::Step::Shares {
                    self.sync_shares();
                    self.render();
                    return;
                }
                if w.step() == create::Step::Codex32 {
                    // The device step is left here rather than at a
                    // finishing step, so the entropy request is cleared
                    // before the plan opens its own runs.
                    self.sync_create();
                    self.sync_codex32();
                    self.render();
                    return;
                }
                // A part is fresh when the step has just been reached,
                // and again each time a part is taken and the next one
                // starts.
                let fresh_part =
                    w.step() == create::Step::XorPart && before != (w.step(), w.xor_parts());
                self.sync_create();
                // A Seed XOR part arrives the way everything else
                // does: the scanner opens on it, and its "Type" row is
                // the word entry (PLANNING §16.88).
                if fresh_part && self.has_camera {
                    self.start_scan(Expect::Seed);
                    self.render();
                    return;
                }
            }
            Wizard::Backup(b) => {
                let (step, key) = (b.step(), b.key());
                let session = &self.session;
                // The words never leave their sealed form except for
                // the length of this call, which is the same rule a
                // key's backup follows.
                let run = |m: &Mnemonic| {
                    let per_page = if step == BackupStep::Steel {
                        views::words::steel_per_page(class, pane, m.language())
                    } else {
                        views::words::per_page(class, pane, m.language())
                    };
                    let grid_pages = osk_ui::components::qr_grid_pages(class);
                    b.tap(
                        id,
                        m.indices(),
                        EntryList::Bip39(m.language()),
                        now,
                        per_page,
                        grid_pages,
                    )
                };
                let done = self
                    .keys
                    .get(key)
                    .and_then(|k| k.mnemonic(session.key(), run));
                match done {
                    None => {
                        self.cancel_wizard();
                        return;
                    }
                    Some(true) => {
                        self.finish_backup();
                        return;
                    }
                    Some(false) => {}
                }
            }
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// The actions that cannot be undone (`docs/PLANNING.md` §16.27):
    /// signing, forgetting one key, wiping every key, and wiping on the
    /// way out. Every other action is a tap, except the one a danger
    /// card turns into a hold: adding a wallet that differs by one key
    /// from a wallet in use (§2.7).
    fn hold(&mut self, id: Id) {
        if id == ids::SIGN_HOLD {
            if self.screen == Screen::Sign {
                let aux = self.schnorr_aux();
                let seed = self.musig_seed();
                let OpenSigner {
                    sign,
                    keys,
                    session,
                    nonce,
                    musig_session,
                    ..
                } = self;
                let masters: Vec<&MasterKey> =
                    keys.iter().filter_map(|k| k.master.as_ref()).collect();
                // A key's scalar is reachable only inside the closure
                // `LoadedKey::secret_share` runs it in, so what the
                // signer is handed is that door and not the secret
                // (§16.103, §16.104 rule 3).
                let session_key = session.key();
                let members: Vec<&LoadedKey> = keys.iter().filter(|k| k.share.is_some()).collect();
                let doors: Vec<Box<osk_psbt::ShareDoor<'_>>> = members
                    .iter()
                    .map(|key| -> Box<osk_psbt::ShareDoor<'_>> {
                        Box::new(move |take: &mut dyn FnMut(&SecShare)| {
                            key.secret_share(session_key, |secret| take(secret));
                        })
                    })
                    .collect();
                let share_keys: Vec<ShareKey> = members
                    .iter()
                    .zip(&doors)
                    .filter_map(|(key, door)| {
                        Some(ShareKey {
                            pubshare: key.share?,
                            open: door.as_ref(),
                        })
                    })
                    .collect();
                sign.sign(&masters, &share_keys, *nonce, aux, musig_session, seed);
                drop(share_keys);
                drop(doors);
                // A session whose every nonce has signed is over, and
                // the badge goes with it.
                if musig_session.as_ref().is_some_and(|s| s.is_empty()) {
                    *musig_session = None;
                }
                self.ui.set_scroll(ids::SCROLL, 0);
            }
        } else if id == ids::MSG_HOLD {
            if self.screen == Screen::SignMessage {
                self.sign_message();
                self.ui.set_scroll(ids::SCROLL, 0);
            }
        } else if id == ids::DETAIL_FORGET {
            if let Screen::Forget(k) = self.screen {
                self.forget(k);
            }
        } else if id == ids::SETTINGS_WIPE {
            // §4.14: the Result is the confirmation the empty Home used
            // to have to carry, and Done leaves it on Home.
            let keys = self.keys.len();
            self.wipe_everything();
            self.stack.clear();
            self.screen = Screen::Wiped(keys);
            self.entered();
        } else if id == ids::SETTINGS_EXIT {
            self.exit();
        } else if id == ids::KEEP_HOLD
            && let Screen::Keep(k) = self.screen
        {
            self.start_keeping(k);
        } else if id == ids::INSPECT_USE_WALLET {
            if self.screen == Screen::Inspect {
                self.use_reviewed_wallet();
            }
        } else if id == ids::BUILD_ADD_WALLET && matches!(self.wizard, Some(Wizard::Build(_))) {
            self.build_add_wallet();
        }
    }

    // ----- opening a key from a loaded key (§16.67) -----

    /// The words of the key at `key`, rebuilt from their sealed form.
    fn key_mnemonic(&self, key: usize) -> Option<Mnemonic> {
        self.keys
            .get(key)?
            .mnemonic(self.session.key(), |m| {
                Mnemonic::from_indices(m.language(), m.indices()).ok()
            })
            .flatten()
    }

    /// How many words a loaded key holds, where it holds any.
    fn key_word_count(&self, key: usize) -> Option<usize> {
        self.keys
            .get(key)?
            .mnemonic(self.session.key(), Mnemonic::word_count)
    }

    /// Adds `key`, which was derived from another loaded key, and says
    /// which key was added. A key with that fingerprint is already the
    /// answer, so nothing is added twice. The screen it was opened from
    /// is replaced, so the chevron goes to the key list from the Result
    /// and from the menu the Result's action opens.
    fn open_derived(&mut self, key: LoadedKey, from: OpenedFrom) {
        let index = match self
            .keys
            .iter()
            .position(|k| k.fingerprint == key.fingerprint)
        {
            Some(i) => i,
            None => {
                let mut key = key;
                key.seal(self.session.key_mut());
                self.keys.push(key);
                self.keys.len() - 1
            }
        };
        match self.return_stack() {
            Some(ReturnTo::Wallet(w)) => {
                self.stack = alloc::vec![
                    Screen::Home,
                    Screen::Wallets,
                    Screen::Wallet(WalletRef::Policy(w)),
                    Screen::WalletKeys(w)
                ];
            }
            // The transaction's Keys review is under the Result, which
            // is the one screen that says which key arrived (§16.110
            // rule 3).
            Some(ReturnTo::Transaction) => {
                let reading = self.key_detour.as_ref().is_some_and(|d| d.reading);
                let under = if reading {
                    Screen::Decode
                } else {
                    Screen::Sign
                };
                self.stack = alloc::vec![Screen::Home, under, Screen::SignKeys(reading)];
            }
            None => {
                self.stack.pop();
            }
        }
        self.screen = Screen::Opened(index, from);
        self.entered();
        // The flow is put back after the screen change, because every
        // screen change clears it.
        self.restore_transaction();
    }

    /// The fingerprint of the key the passphrase typed so far opens,
    /// derived after every keystroke so that the screen states which key
    /// ✓ would add (§16.67). An empty passphrase derives nothing: it is
    /// the key that is already loaded.
    fn build_open_passphrase(&mut self, key: usize) {
        let Some(mnemonic) = self.key_mnemonic(key) else {
            return;
        };
        let network = self.network;
        if let Some(f) = self.open_finish.as_mut() {
            f.build(&Material::Words(&mnemonic), network);
        }
    }

    /// The fingerprint of the child the index typed so far derives,
    /// after every keystroke (§16.67). An empty or out-of-range index
    /// derives nothing.
    fn build_open_child(&mut self, key: usize) {
        let fingerprint = self.child_fingerprint(key);
        if let Some(c) = self.child.as_mut() {
            c.fingerprint = fingerprint;
        }
    }

    /// The master fingerprint of the BIP-85 child the word count and
    /// index now hold name.
    fn child_fingerprint(&self, key: usize) -> Option<Fingerprint> {
        let c = self.child.as_ref()?;
        let index = c.index_value()?;
        let k = self.keys.get(key)?;
        let master = k.master.as_ref()?;
        let child = osk_bip::bip85::child_mnemonic(master, k.language, c.words, index).ok()?;
        let seed = child.to_seed(b"").ok()?;
        Some(MasterKey::from_seed(&seed, self.network).fingerprint())
    }

    /// ✓ on "Open passphrase": the key of the parent's words and the
    /// passphrase typed here. An empty passphrase is the parent's own
    /// key, so ✓ does nothing until something is typed.
    fn open_passphrase_done(&mut self, key: usize) {
        if !self
            .open_finish
            .as_ref()
            .is_some_and(Finish::has_passphrase)
        {
            return;
        }
        let Some(mnemonic) = self.key_mnemonic(key) else {
            return;
        };
        let verified = self.keys.get(key).is_some_and(|k| k.backup_verified);
        let network = self.network;
        let Some(f) = self.open_finish.as_mut() else {
            return;
        };
        if !f.build(&Material::Words(&mnemonic), network) {
            return;
        }
        let Some(opened) = f.take_key(Some(mnemonic), verified) else {
            return;
        };
        let opened = opened.derived();
        // A key made from a wallet's not-loaded row is that wallet's key
        // or it is nothing (§16.104 rule 6). The passphrase can be typed
        // again; nothing about this one is kept, and the field keeps
        // what it holds.
        let wrong = self.strings().open_wrong_passphrase;
        if self.wrong_key(&opened, wrong) {
            drop(opened);
            self.build_open_passphrase(key);
            return;
        }
        self.open_finish = None;
        self.mask_deadline = None;
        if let Some(w) = self.build_parked.take() {
            self.build_passphrase_opened(opened, w);
            return;
        }
        self.open_derived(opened, OpenedFrom::Passphrase);
    }

    /// The word-count choice of "Open BIP-85 child seed", and the
    /// Continue that goes on to the index.
    fn tap_open_child(&mut self, id: Id) {
        let Some(c) = self.child.as_mut() else {
            return;
        };
        if c.step != ChildStep::Words {
            return;
        }
        if let Some(i) = ids::index_in(id, ids::OPEN_CHILD_WORDS_BASE, CHILD_COUNTS.len()) {
            c.words = CHILD_COUNTS[i];
        } else if id == ids::OPEN_CHILD_CONTINUE {
            c.step = ChildStep::Index;
            self.ui.set_scroll(ids::SCROLL, 0);
        }
    }

    /// ✓ on the child's index: the child mnemonic of the parent's
    /// master, loaded as a key of its own.
    fn open_child_done(&mut self, key: usize) {
        let Some(c) = self.child.as_ref() else {
            return;
        };
        if c.step != ChildStep::Index {
            return;
        }
        let (words, Some(index)) = (c.words, c.index_value()) else {
            return;
        };
        let network = self.network;
        let child = self.keys.get(key).and_then(|k| {
            let master = k.master.as_ref()?;
            osk_bip::bip85::child_mnemonic(master, k.language, words, index).ok()
        });
        let Some(child) = child else {
            return;
        };
        let Ok(seed) = child.to_seed(b"") else {
            return;
        };
        let Some(seed) = SeedBytes::new(seed.expose()).map(Secret::new) else {
            return;
        };
        let master = MasterKey::from_seed_bytes(&seed, network);
        let opened = LoadedKey::new(seed, master, Some(child), false, false).derived();
        // The same check as the passphrase's: a child made from a
        // wallet's row is that wallet's key or it is nothing.
        let wrong = self.strings().open_wrong_child;
        if self.wrong_key(&opened, wrong) {
            return;
        }
        self.child = None;
        self.open_derived(opened, OpenedFrom::Child);
    }

    /// A digit on one of the BIP-85 row's two pads: the length of a
    /// password, or the index. ✓ goes on to the next step, and on the
    /// index it opens the Secret screen the value is drawn on
    /// (`docs/PLANNING.md` §16.114).
    fn bip85_typed(&mut self, id: Id, input: KeyInput) {
        let Some(flow) = self.bip85.as_mut() else {
            return;
        };
        let length = id == ids::BIP85_LENGTH_PAD;
        if length != (flow.step == Bip85Step::Length) {
            return;
        }
        // Two digits hold 86, the longest password either application
        // writes; the index takes what the largest hardened child is
        // written in.
        let room = if length { 2 } else { MAX_CHILD_INDEX_CHARS };
        let field = if length {
            &mut flow.length
        } else {
            &mut flow.index
        };
        match input {
            KeyInput::Char(c) if c.is_ascii_digit() && field.len() < room => field.push(c),
            KeyInput::Backspace => {
                field.pop();
            }
            KeyInput::Done => {
                if length {
                    if flow.length_value().is_some() {
                        flow.step = Bip85Step::Index;
                    }
                } else if flow.index_value().is_some() {
                    flow.step = Bip85Step::Value;
                }
                self.ui.set_scroll(ids::SCROLL, 0);
            }
            KeyInput::Char(_) | KeyInput::Shift | KeyInput::Symbols => {}
        }
    }

    fn keyboard(&mut self, id: Id, input: KeyInput) {
        if id == ids::KEEP_DURESS_KEYBOARD && self.screen == Screen::DuressPin {
            self.duress_input(input);
            return;
        }
        let now = self.now_ms;
        let network = self.network;
        if id == ids::VANITY_KEYBOARD && matches!(self.screen, Screen::Vanity(_)) {
            self.vanity_typed(input);
            return;
        }
        if id == ids::OPEN_PASS_KEYBOARD
            && let Screen::OpenPassphrase(key) = self.screen
        {
            match input {
                KeyInput::Char(c) => {
                    self.open_error = None;
                    if let Some(f) = self.open_finish.as_mut() {
                        f.passphrase_push(c, now);
                    }
                    self.build_open_passphrase(key);
                }
                KeyInput::Backspace => {
                    self.open_error = None;
                    if let Some(f) = self.open_finish.as_mut() {
                        f.passphrase_pop();
                    }
                    self.build_open_passphrase(key);
                }
                KeyInput::Done => self.open_passphrase_done(key),
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            self.mask_deadline = self.open_finish.as_ref().and_then(Finish::mask_deadline);
            return;
        }
        if id == ids::OPEN_CHILD_KEYBOARD
            && let Screen::OpenChild(key) = self.screen
        {
            match input {
                KeyInput::Char(c) => {
                    self.open_error = None;
                    if let Some(f) = self.child.as_mut()
                        && f.step == ChildStep::Index
                        && c.is_ascii_digit()
                        && f.index.len() < MAX_CHILD_INDEX_CHARS
                    {
                        f.index.push(c);
                    }
                    self.build_open_child(key);
                }
                KeyInput::Backspace => {
                    self.open_error = None;
                    if let Some(f) = self.child.as_mut() {
                        f.index.pop();
                    }
                    self.build_open_child(key);
                }
                KeyInput::Done => self.open_child_done(key),
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            return;
        }
        if (id == ids::BIP85_LENGTH_PAD || id == ids::BIP85_INDEX_PAD)
            && matches!(self.screen, Screen::Bip85(_))
        {
            self.bip85_typed(id, input);
            return;
        }
        if id == ids::LIGHTNING_PASS_KEYBOARD
            && self.screen == Screen::Lightning
            && self.wizard.is_none()
        {
            let mut resolve = false;
            if let Some(flow) = self.lightning.as_mut() {
                match input {
                    KeyInput::Char(c) => {
                        flow.passphrase_push(c, now);
                    }
                    KeyInput::Backspace => flow.passphrase_pop(),
                    KeyInput::Done => resolve = true,
                    KeyInput::Shift | KeyInput::Symbols => {}
                }
            }
            if resolve {
                if let Some(flow) = self.lightning.as_mut() {
                    flow.resolve_aezeed(network);
                }
                self.ui.set_scroll(ids::SCROLL, 0);
            }
            self.mask_deadline = self.lightning.as_ref().and_then(Lightning::mask_deadline);
            return;
        }
        if (id == ids::EXPLORE_PATH_KEYBOARD || id == ids::EXPLORE_PASS_KEYBOARD)
            && self.screen == Screen::Explore
            && self.wizard.is_none()
        {
            let e = &mut self.explore;
            match (id == ids::EXPLORE_PATH_KEYBOARD, input) {
                (true, KeyInput::Char(c)) => {
                    e.path_push(c);
                }
                (true, KeyInput::Backspace) => e.path_pop(),
                (true, KeyInput::Done) => {
                    e.apply();
                }
                (false, KeyInput::Char(c)) => {
                    e.passphrase_push(c, now, network);
                }
                (false, KeyInput::Backspace) => e.passphrase_pop(network),
                (false, KeyInput::Done) => e.go(explore::Step::Keys),
                (_, KeyInput::Shift | KeyInput::Symbols) => {}
            }
            self.mask_deadline = e.mask_deadline();
            return;
        }
        if id == ids::WORDLIST_KEYBOARD && self.screen == Screen::WordList {
            if self.word_list.by() == wordlist::SearchBy::Word {
                let w = self.word_list.search_mut();
                match input {
                    KeyInput::Char(c) => {
                        w.type_char(c);
                    }
                    KeyInput::Backspace => w.backspace(),
                    KeyInput::Done => {
                        w.commit_selected();
                    }
                    KeyInput::Shift | KeyInput::Symbols => {}
                }
                self.word_search_settled();
            } else {
                match input {
                    KeyInput::Char(c) => {
                        self.word_list.push(c);
                    }
                    KeyInput::Backspace => self.word_list.pop(),
                    // §4.3: ✓ is dead until what is typed names a word,
                    // so a hardware Enter does nothing either.
                    KeyInput::Done => {
                        if let Some(i) = self.word_list.index() {
                            self.word_list.clear();
                            self.push(Screen::Word(i));
                        }
                    }
                    KeyInput::Shift | KeyInput::Symbols => {}
                }
            }
            return;
        }
        if id == ids::TOOL_KEYBOARD
            && let Screen::Tool(tool) = self.screen
        {
            match input {
                KeyInput::Char(c) => {
                    self.calc.push(tool, c);
                }
                KeyInput::Backspace => self.calc.pop(),
                // §4.3: ✓ is dead until the field says something the
                // tool can answer, so a hardware Enter does nothing
                // either. Units has no ✓ at all: its screen is the
                // answer.
                KeyInput::Done => {
                    if self
                        .calc
                        .ready(tool, self.network, &self.policy_key_expressions())
                    {
                        self.push(Screen::ToolResult(tool));
                    }
                }
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            return;
        }
        if id == ids::DICE_PAD && self.screen == Screen::DicePassphrase {
            match input {
                KeyInput::Char(c) => {
                    self.dice.push(c, now);
                }
                KeyInput::Backspace => self.dice.pop(),
                KeyInput::Done => {
                    if self.dice.ready() {
                        self.dice.go(dice::Step::Result);
                        self.ui.set_scroll(ids::SCROLL, 0);
                    }
                }
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            self.mask_deadline = self.dice.mask_deadline();
            return;
        }
        if id == ids::SCAN_PASS_KEYBOARD
            && self.screen == Screen::Scan
            && self.scan_stage() == Some(ScanStage::BackupPassphrase)
        {
            match input {
                KeyInput::Char(c) => {
                    if let Some(s) = self.scan.as_mut() {
                        s.backup_push(c, now);
                    }
                }
                KeyInput::Backspace => {
                    if let Some(s) = self.scan.as_mut() {
                        s.backup_pop();
                    }
                }
                KeyInput::Done => self.open_scanned_backup(),
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            self.mask_deadline = self.scan.as_ref().and_then(ScanState::mask_deadline);
            return;
        }
        // A codex32 string being typed back.
        if id == ids::CODEX32_KEYBOARD
            && self.on_codex32_plan()
            && self.codex32.step() == Codex32Step::TypeBack
        {
            let next = self.codex32.key(input);
            self.after_codex32(next);
            return;
        }
        // The passphrase a SLIP-39 backup's shares are written under.
        if id == ids::LOAD_PASS_KEYBOARD
            && self.on_share_plan()
            && self.shares.step() == ShareStep::Passphrase
        {
            let next = self.shares.key(input, now);
            self.mask_deadline = self.shares.mask_deadline();
            self.after_shares(next);
            return;
        }
        if id == ids::BACKUP_PASS_KEYBOARD
            && let Some(Wizard::Backup(b)) = &mut self.wizard
            && matches!(
                b.step(),
                BackupStep::Passphrase | BackupStep::PassphraseRepeat
            )
        {
            if b.pass_key(input, now) {
                self.make_backup();
            }
            self.mask_deadline = match &self.wizard {
                Some(Wizard::Backup(b)) => b.mask_deadline(),
                _ => None,
            };
            self.ui.set_scroll(ids::SCROLL, 0);
            return;
        }
        // §16.110 rule 2: the session token, on the hex keyboard, with
        // ✓ live on `00` or a nonce of BIP 129's two lengths.
        if id == ids::BSMS_TOKEN_KEYBOARD
            && let Screen::KeyExport(key, account, KeyExportStep::Token) = self.screen
        {
            match input {
                KeyInput::Char(c) => {
                    if self.bsms.token.len() < 32 {
                        self.bsms.token.push(c.to_ascii_lowercase());
                    }
                }
                KeyInput::Backspace => {
                    self.bsms.token.pop();
                }
                KeyInput::Done => {
                    if bsms::is_token(&self.bsms.token) {
                        self.screen = Screen::KeyExport(key, account, KeyExportStep::Description);
                        self.ui.set_scroll(ids::SCROLL, 0);
                    }
                }
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            return;
        }
        // The description, on the name keyboard, at most the eighty
        // characters BIP 129 allows.
        if id == ids::BSMS_DESCRIPTION_KEYBOARD
            && let Screen::KeyExport(key, account, KeyExportStep::Description) = self.screen
        {
            match input {
                KeyInput::Char(c) => {
                    if self.bsms.description.chars().count() < BSMS_DESCRIPTION_TYPED {
                        self.bsms.description.push(c);
                    }
                }
                KeyInput::Backspace => {
                    self.bsms.description.pop();
                }
                KeyInput::Done => {
                    if self.bsms.description.chars().count() <= BSMS_DESCRIPTION_MAX {
                        self.write_signer_record(key, account);
                        self.screen = Screen::KeyExport(key, account, KeyExportStep::Export);
                        self.ui.set_scroll(ids::SCROLL, 0);
                    }
                }
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            return;
        }
        // §16.113: the user name and the domain BIP-353's record is
        // published under, one after the other, and then the record
        // itself back on the export.
        if id == ids::SILENT_DNS_KEYBOARD
            && let Screen::SilentDns(_) = self.screen
        {
            match input {
                KeyInput::Char(c) => self.dns.push(c),
                KeyInput::Backspace => self.dns.pop(),
                KeyInput::Done => {
                    if self.dns.field().trim().is_empty() {
                        return;
                    }
                    match self.dns.step {
                        silent::DnsStep::User => {
                            self.dns.step = silent::DnsStep::Domain;
                            self.ui.set_scroll(ids::SCROLL, 0);
                        }
                        silent::DnsStep::Domain => {
                            self.back();
                            // Leaving the entry re-enters the export,
                            // which opens on the first format a silent
                            // payments wallet has; the record is the
                            // format that was chosen, so it is put back.
                            if matches!(self.screen, Screen::Export(_)) {
                                self.export.format = ExportFormat::ALL
                                    .iter()
                                    .position(|f| *f == ExportFormat::SilentDns)
                                    .unwrap_or(0);
                            }
                        }
                    }
                }
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            return;
        }
        // A note being typed, whether it is a free-standing note or a
        // recovery sheet's (`docs/PLANNING.md` §16.112 rule 2).
        if id == ids::NOTE_KEYBOARD && self.screen == Screen::NoteText {
            match input {
                KeyInput::Char(c) => self.note.push(c),
                KeyInput::Backspace => self.note.pop(),
                KeyInput::Done => {
                    if !self.note.is_empty() {
                        self.note_typed();
                    }
                }
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            return;
        }
        // The passphrase a note or a sheet is sealed under, typed twice.
        if id == ids::SEAL_PASS_KEYBOARD && self.screen == Screen::SealPass {
            match input {
                KeyInput::Char(c) => self.seal_pass.push(c, self.now_ms),
                KeyInput::Backspace => self.seal_pass.pop(),
                KeyInput::Done => {
                    if self.seal_pass.done() {
                        self.seal_now();
                    }
                }
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            self.mask_deadline = self.seal_pass.mask_deadline();
            return;
        }
        if id == ids::WALLET_NAME_KEYBOARD
            && let Screen::WalletName(wallet) = self.screen
        {
            match input {
                KeyInput::Char(c) => {
                    if self.name_entry.chars().count() < osk_keep::names::MAX_NAME {
                        self.name_entry.push(c);
                    }
                }
                KeyInput::Backspace => {
                    self.name_entry.pop();
                }
                // An empty field takes the name away, so ✓ is live
                // whatever is in it.
                KeyInput::Done => {
                    let name = core::mem::take(&mut self.name_entry);
                    self.set_wallet_name(wallet, &name);
                    self.back();
                }
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            return;
        }
        // The builder's "Type" entry: the same text the file and the QR
        // routes read, typed.
        if id == ids::BUILD_DAYS_PAD {
            let next = match &mut self.wizard {
                Some(Wizard::Build(w)) if w.step() == build::Step::Days => match input {
                    KeyInput::Char(c) => {
                        w.days_push(c);
                        None
                    }
                    KeyInput::Backspace => {
                        w.days_pop();
                        None
                    }
                    // §4.3: ✓ is dead while the number is no wait this
                    // wallet can hold, so a hardware Enter does nothing
                    // either.
                    KeyInput::Done if w.days_ready() => {
                        w.take_typed_days();
                        Some(w.after_delay())
                    }
                    KeyInput::Done | KeyInput::Shift | KeyInput::Symbols => None,
                },
                _ => None,
            };
            if let Some(next) = next {
                self.with_build(|w| w.go(next));
            }
            return;
        }
        if id == ids::BUILD_TYPE_KEYBOARD {
            let typed = match &mut self.wizard {
                Some(Wizard::Build(w)) if w.step() == build::Step::Type => match input {
                    KeyInput::Char(c) => {
                        w.type_push(c);
                        None
                    }
                    KeyInput::Backspace => {
                        w.type_pop();
                        None
                    }
                    KeyInput::Done => Some(String::from(w.typed())),
                    KeyInput::Shift | KeyInput::Symbols => None,
                },
                _ => None,
            };
            if let Some(text) = typed {
                self.cosigner_scanned(&text);
            }
            return;
        }
        if id == ids::MSG_KEYBOARD {
            if let Some(flow) = self.message.as_mut()
                && flow.stage() == message::Stage::Typing
            {
                match input {
                    KeyInput::Char(c) => flow.type_push(c),
                    KeyInput::Backspace => flow.type_pop(),
                    KeyInput::Done => {
                        flow.typed();
                        self.ui.set_scroll(ids::SCROLL, 0);
                    }
                    KeyInput::Shift | KeyInput::Symbols => {}
                }
            }
            return;
        }
        if id == ids::VERIFY_KEYBOARD {
            if self.screen == Screen::Verify && self.verify.stage() == VerifyStage::Typing {
                match input {
                    KeyInput::Char(c) => {
                        self.verify.push(c);
                        self.address_layer();
                    }
                    KeyInput::Backspace => {
                        self.verify.pop();
                        self.address_layer();
                    }
                    // §4.3: ✓ is dead while the text is not an
                    // address, so a hardware Enter does nothing either.
                    KeyInput::Done if self.verify.parses() => {
                        let text = String::from(self.verify.input());
                        let result = self.check_address(&text);
                        self.verify.set_result(&text, result);
                        self.ui.set_scroll(ids::SCROLL, 0);
                    }
                    KeyInput::Done => {}
                    KeyInput::Shift | KeyInput::Symbols => {}
                }
            }
            return;
        }
        let Some(w) = &mut self.wizard else {
            return;
        };
        self.mask_deadline = match w {
            Wizard::Load(w) => {
                w.key(id, input, network, now);
                w.mask_deadline()
            }
            Wizard::Create(w) => {
                w.key(id, input, network, now);
                let deadline = w.mask_deadline();
                self.sync_create();
                deadline
            }
            Wizard::Backup(b) => match b.gather_mut() {
                Some(g) => {
                    g.key(id, input, network, now);
                    let deadline = g.mask_deadline();
                    self.sync_create();
                    deadline
                }
                None => None,
            },
            Wizard::Build(_) => None,
        };
        if id == ids::LOAD_PIN_KEYBOARD {
            self.pin_set();
        }
    }

    // ----- Tools > Word list -----

    /// A word the search settled on opens its own screen, and the field
    /// is empty again for the next search.
    fn word_search_settled(&mut self) {
        let found = self.word_list.search().committed_indices().next();
        if let Some(i) = found {
            self.word_list.clear();
            self.push(Screen::Word(i));
        }
    }

    /// A calculator's field: the mode row opens its Choice, and the
    /// keyboard does the rest.
    fn tap_tool(&mut self, id: Id, tool: Tool) {
        if id != ids::TOOL_MODE {
            return;
        }
        match tool {
            Tool::Hashes => {
                let picked = osk_codec::encodings::ReadAs::ALL
                    .iter()
                    .position(|r| *r == self.calc.read_as())
                    .unwrap_or(0);
                self.overlay = Some(Overlay::Choice(Picker::ReadAs, picked));
            }
            Tool::Units => {
                let picked = osk_ui::components::Denomination::ALL
                    .iter()
                    .position(|u| *u == self.calc.from())
                    .unwrap_or(0);
                self.overlay = Some(Overlay::Choice(Picker::FromUnit, picked));
            }
            Tool::Miniscript => {
                let picked = osk_bip::compile::PolicyScript::ALL
                    .iter()
                    .position(|script| *script == self.calc.script())
                    .unwrap_or(0);
                self.overlay = Some(Overlay::Choice(Picker::PolicyScript, picked));
            }
            _ => {}
        }
    }

    /// A calculator's answer: every long string is a reference row that
    /// opens Compare (§4.5).
    fn tap_tool_result(&mut self, id: Id, tool: Tool) {
        // A compiled policy whose keys are all extended public keys with
        // an origin is a wallet, and the way on is the review every
        // other wallet gets.
        if id == ids::TOOL_LOAD_WALLET {
            let policy = match tool {
                Tool::Descriptor => {
                    osk_bip::descriptor::checksum_facts(self.calc.typed()).and_then(|f| f.wallet)
                }
                _ => self.policy_facts().and_then(|f| f.wallet),
            };
            if let Some(policy) = policy {
                self.review_wallet(policy);
                return;
            }
        }
        let found = self
            .tool_strings(tool)
            .into_iter()
            .find(|(at, _, _)| ids::at(ids::TOOL_ROW_BASE, *at) == id);
        if let Some((_, label, value)) = found {
            self.open_compare(label, value);
        }
    }

    /// Decode a transaction: the same review taps the Sign flow answers,
    /// with Done in place of the confirm.
    fn tap_decode(&mut self, id: Id) {
        if id == ids::DECODE_DONE {
            self.go_home();
            return;
        }
        let Stage::Wizard(step) = self.decode.stage() else {
            if id == ids::SIGN_BACK {
                self.back();
            }
            return;
        };
        if id == ids::SIGN_KEYS {
            self.open_transaction_keys(true);
            return;
        }
        match step {
            sign::Step::Outputs => {
                if id == ids::SIGN_CONTINUE {
                    if self.decode.next_output() {
                        return;
                    }
                    self.decode_next(step);
                } else if let Some(o) = self
                    .decode
                    .inspection()
                    .and_then(|i| i.outputs.get(self.decode.output()))
                    && id == ids::at(ids::SIGN_OUT_BASE, o.index)
                {
                    let (label, address) = (self.strings().sign_to, o.address.clone());
                    self.open_compare(label, address);
                }
            }
            // The Signatures page, which a read transaction reaches
            // from its own review (§16.111).
            sign::Step::Signatures => {
                self.compare_signature(id, true);
            }
            _ => {
                if id == ids::SIGN_CONTINUE {
                    self.decode_next(step);
                } else if id == ids::SIGN_SIGNATURES && step == sign::Step::Inputs {
                    self.decode.open_signatures(step);
                } else if id == ids::SIGN_TO || id == ids::SIGN_TXID {
                    self.compare_decode_string(id);
                }
            }
        }
    }

    /// The recipient and the transaction id of the transaction being
    /// read, as Compare strings.
    fn compare_decode_string(&mut self, id: Id) {
        let s = self.strings();
        let found = if id == ids::SIGN_TXID {
            self.decode
                .unsigned_txid()
                .map(|t| (s.sign_txid, alloc::format!("{t}")))
        } else {
            self.decode
                .inspection()
                .and_then(|i| i.outputs.iter().find(|o| !o.is_ours()))
                .map(|o| (s.sign_to, o.address.clone()))
        };
        if let Some((label, value)) = found {
            self.open_compare(label, value);
        }
    }

    /// The next page of the reading review.
    fn decode_next(&mut self, step: sign::Step) {
        if let Some(next) = self.decode.after(step) {
            self.decode.go(next);
            if next == sign::Step::Outputs {
                self.decode.set_output(0);
            }
            self.ui.set_scroll(ids::SCROLL, 0);
        }
    }

    fn tap_word_list(&mut self, id: Id) {
        match self.word_list.step() {
            wordlist::Step::Language => {
                if let Some(i) = ids::index_in(id, ids::WORDLIST_LANG_BASE, Language::ALL.len()) {
                    self.word_list.set_language(Language::ALL[i]);
                } else if id == ids::WORDLIST_LANG_CONTINUE {
                    self.word_list.go(wordlist::Step::Search);
                    self.ui.set_scroll(ids::SCROLL, 0);
                }
            }
            wordlist::Step::Search => {
                if id == ids::WORDLIST_BY {
                    let picked = wordlist::SearchBy::ALL
                        .iter()
                        .position(|b| *b == self.word_list.by())
                        .unwrap_or(0);
                    self.overlay = Some(Overlay::Choice(Picker::SearchBy, picked));
                } else if id == ids::WORDLIST_CANDIDATES {
                    // The lone candidate is one chip and reports its own
                    // id, as it does in the Load wizard (§4.3).
                    self.word_list.search_mut().commit_candidate(0);
                    self.word_search_settled();
                }
            }
        }
    }

    /// The pager walks the list in place: one word's screen is the next
    /// word's screen, so Back returns to the search and not through
    /// every word that was read.
    fn tap_word(&mut self, id: Id, index: u16) {
        let next = if id == ids::WORD_PREV {
            index.checked_sub(1)
        } else if id == ids::WORD_NEXT {
            (usize::from(index) + 1 < wordlist::WORDS).then_some(index + 1)
        } else {
            None
        };
        if let Some(i) = next {
            self.screen = Screen::Word(i);
            self.ui.set_scroll(ids::SCROLL, 0);
        }
    }

    // ----- Tools > Dice passphrase -----

    fn tap_dice(&mut self, id: Id) {
        match self.dice.step() {
            dice::Step::List => {
                if let Some(i) = ids::index_in(id, ids::DICE_LIST_BASE, List::ALL.len()) {
                    self.dice.set_list(List::ALL[i]);
                } else if id == ids::DICE_LIST_CONTINUE {
                    self.dice.go(dice::Step::Words);
                    self.ui.set_scroll(ids::SCROLL, 0);
                }
            }
            dice::Step::Words => {
                if let Some(i) = ids::index_in(id, ids::DICE_WORDS_BASE, dice::WORD_COUNTS.len()) {
                    self.dice.set_words(i);
                } else if id == ids::DICE_WORDS_CONTINUE {
                    self.dice.go(dice::Step::Rolls);
                    self.ui.set_scroll(ids::SCROLL, 0);
                }
            }
            dice::Step::Rolls => {
                if id == ids::DICE_CONTINUE && self.dice.ready() {
                    self.dice.go(dice::Step::Result);
                    self.ui.set_scroll(ids::SCROLL, 0);
                }
            }
            // Nothing is stored: the passphrase was never written
            // anywhere, and leaving drops the rolls it came from.
            dice::Step::Result => {
                if id == ids::DICE_DONE {
                    self.dice = DicePassphrase::new();
                    self.back();
                }
            }
            dice::Step::Discard => {
                if id == ids::DICE_DISCARD {
                    self.dice = DicePassphrase::new();
                    self.back();
                } else if id == ids::DICE_KEEP {
                    self.dice.keep();
                }
            }
        }
    }

    // ----- rendering -----

    fn build(&self) -> Node {
        if let Some(Err(check)) = self.selftest {
            return self.view_selftest_failed(check);
        }
        if self.boot_refused() {
            return self.view_boot_refused();
        }
        if self.session.is_locked() {
            return self.view_lock();
        }
        if self.front_door() {
            return self.view_stored_key();
        }
        self.build_screen()
    }

    fn build_screen(&self) -> Node {
        // §16.105: a page opened from the info button is drawn over the
        // screen that opened it, wizard and all, because the wizard
        // lives beside the stack and is not left.
        if let Screen::LearnTopic(page, section) = self.screen {
            return self.view_learn_topic(page, section);
        }
        if let Some(o) = &self.overlay {
            return self.view_overlay(o);
        }
        if self.building_scan() {
            return self.view_scan();
        }
        match &self.wizard {
            Some(Wizard::Load(w)) => return self.view_load(w),
            Some(Wizard::Build(w)) => return self.view_build(w),
            Some(Wizard::Create(w)) if w.step() == create::Step::Shares => {
                return self.view_shares();
            }
            Some(Wizard::Create(w)) if w.step() == create::Step::Codex32 => {
                return self.view_codex32();
            }
            Some(Wizard::Create(w)) => return self.view_create(w),
            Some(Wizard::Backup(b)) => {
                return match b.gather() {
                    Some(g) => self.view_create(g),
                    None if b.step() == BackupStep::Shares => self.view_shares(),
                    None if b.step() == BackupStep::Codex32 => self.view_codex32(),
                    None => self.view_backup(b),
                };
            }
            None => {}
        }
        match self.screen {
            Screen::Home if self.start_here_shown() => self.view_start_here(),
            Screen::Home => self.view_home(),
            Screen::StartHere => self.view_start_here(),
            Screen::Created(key) => self.view_created(key),
            Screen::Keys => self.view_keys(),
            Screen::Wallets => self.view_wallets(),
            Screen::Add => self.view_add(),
            Screen::AddWallet => self.view_add_wallet(),
            Screen::PickKey(purpose) => self.view_pick_key(purpose),
            Screen::WalletKeys(w) => self.view_wallet_keys(w),
            Screen::KeyDetail(k) => self.view_detail(k),
            Screen::Wallet(w) => self.view_wallet(w),
            Screen::WalletName(w) => self.view_wallet_name(w),
            Screen::Addresses(owner) => self.view_addresses(owner),
            Screen::BackupMenu(k) => self.view_backup_menu(k),
            Screen::OpenPassphrase(k) => self.view_open_passphrase(k),
            Screen::OpenChild(k) => self.view_open_child(k),
            Screen::Bip85(k) => self.view_bip85(k),
            Screen::Vanity(k) => self.view_vanity(k),
            Screen::Lightning => self.view_lightning(),
            Screen::Opened(k, from) => self.view_opened(k, from),
            Screen::Forget(k) => self.view_forget(k),
            Screen::Keep(k) => self.view_keep(k),
            Screen::DuressPin => self.view_duress_pin(),
            Screen::KeptRemoved => self.view_kept_removed(),
            Screen::Settings => self.view_settings(),
            Screen::About => self.view_about(),
            Screen::Sign => self.view_sign(&self.sign),
            Screen::Scan => self.view_scan(),
            Screen::Verify => self.view_verify(),
            Screen::SignMessage => match &self.message {
                Some(flow) => self.view_sign_message(flow),
                None => self.view_home(),
            },
            Screen::CheckedMessage => match &self.check {
                Some(check) => self.view_check_message(check),
                None => self.view_home(),
            },
            Screen::Export(owner) => self.view_export(Exported::Wallet(owner)),
            Screen::KeyExport(key, account, step) => self.view_key_export(key, account, step),
            Screen::SignKeys(reading) => self.view_sign_keys(reading),
            Screen::Inspect => self.view_inspect(),
            Screen::Explore => self.view_explore(),
            Screen::Tools => self.view_tools(),
            Screen::WordList => self.view_word_list(&self.word_list),
            Screen::Word(i) => self.view_word(self.word_list.language(), i),
            Screen::DicePassphrase => self.view_dice(&self.dice),
            Screen::Tool(t) => self.view_tool(t),
            Screen::ToolResult(t) => self.view_tool_result(t),
            Screen::Decode => self.view_sign(&self.decode),
            Screen::CompareTx => self.view_compare_tx(),
            Screen::Notes => self.view_notes(),
            Screen::NoteText => self.view_note_text(),
            Screen::Note => self.view_note(),
            Screen::Sheet(w) => self.view_sheet(w),
            Screen::SilentAddress(w, label) => self.view_silent_address(w, label),
            Screen::SilentLabels(w) => self.view_silent_labels(w),
            Screen::SilentCheck(w) => self.view_silent_check(w),
            Screen::SilentDns(w) => self.view_silent_dns(w),
            Screen::SilentSecret(w) => self.view_silent_secret(w),
            Screen::OpenedSheet => self.view_opened_sheet(),
            Screen::ExportForm => self.view_export_form(),
            Screen::SealPass => self.view_seal_pass(),
            Screen::Sealed => self.view_sealed(),
            Screen::SealedQr => self.view_sealed_qr(),
            Screen::Tiers => self.view_tiers(),
            Screen::Learn => self.view_learn(),
            Screen::LearnPage(p) => self.view_learn_page(p),
            Screen::LearnTopic(p, section) => self.view_learn_topic(p, section),
            Screen::WipeAll => self.view_wipe_all(),
            Screen::Wiped(n) => self.view_wiped(n),
            Screen::WipeAndExit => self.view_wipe_and_exit(),
            Screen::Ended => self.view_session_ended(),
            Screen::NoSecureBoot => self.view_no_secure_boot(),
        }
    }

    /// Derives every loaded key's account keys, so that a scanned public
    /// key can be told from the keys in memory (§4.8 Mine). Public data
    /// only, and only for the one screen that compares them.
    fn prepare_inspect(&mut self) {
        let xpub = self.strings().inspect_xpub;
        if self.inspect.as_ref().is_none_or(|d| d.title != xpub) {
            return;
        }
        for i in 0..self.keys.len() {
            for script in ScriptType::ALL {
                self.account(i, script);
            }
        }
    }

    fn render(&mut self) {
        let Some(ctx) = self.ctx else {
            return;
        };
        // The node-key tool's word entry is a wizard, so its words are
        // taken over before the wizard gate below (§16.116).
        if self.screen == Screen::Lightning {
            self.take_aezeed_words();
        }
        if self.wizard.is_none() {
            match self.screen {
                Screen::Addresses(WalletRef::Key(k)) => self.prepare_detail(k),
                Screen::Addresses(WalletRef::Typed) => self.prepare_typed_addresses(),
                Screen::Addresses(WalletRef::Policy(w)) => self.prepare_wallet_addresses(w),
                Screen::Export(WalletRef::Key(k)) => {
                    let script = self.export.script;
                    self.account(k, script);
                }
                Screen::Export(WalletRef::Policy(_)) => {}
                // A key's account was derived when it was chosen, so
                // nothing is derived per frame here.
                Screen::KeyExport(..) | Screen::SignKeys(_) => {}
                // The single-sig wallet's Keys review is its key's
                // account descriptor, which has to be derived first.
                Screen::Wallet(WalletRef::Key(k)) => {
                    let script = self.detail.script;
                    self.account(k, script);
                }
                Screen::Wallet(WalletRef::Policy(_)) => {}
                Screen::SignMessage => self.prepare_message(),
                Screen::Explore => self.prepare_explore(),
                Screen::Lightning => self.prepare_lightning(),
                Screen::Inspect => self.prepare_inspect(),
                _ => {}
            }
        }
        let tree = self.build();
        self.reveal_second = self.reveal_left_s();
        self.qr_visible = qr_visible_in(&tree, &self.ui);
        let Some(canvas) = self.canvas.as_ref() else {
            return;
        };
        let layout = layout::solve_with(&tree, canvas.bounds(), &ctx, self.ui.scroll_offsets());
        // §4.15: the ring is on an item of the frame about to be drawn,
        // or on nothing, which is what a screen change leaves.
        self.ui.retain_focus(&layout);
        // §16.119 rule 5: the tree is kept beside the layout and painted
        // when the frame is read, so a shell paints once per frame it
        // draws rather than once per event.
        self.layout = Some(layout);
        self.tree = Some(tree);
        self.dirty = true;
        // §4.6: a screen that opens with something below the fold — the
        // path editor's checked preset — scrolls it into view once the
        // frame that placed it exists, and draws once.
        if self.scrolled_into_view() {
            return;
        }
        self.commands.push_back(Command::Draw);
    }

    /// Draws the frame a scroll asks for by moving the pixels that are
    /// already on the panel (PLANNING §16.36): the content is what the
    /// last frame drew, one offset further on. The region's pixels move
    /// inside its viewport, the strip that came into view and the
    /// scrollbar's track are drawn again, and nothing else is solved or
    /// painted. `false` when the frame is more than that, and the caller
    /// renders.
    fn scroll_moved(&mut self, id: Id) -> bool {
        if !self.pixels_can_move() {
            return false;
        }
        let (Some(ctx), Some(layout)) = (self.ctx, self.layout.as_mut()) else {
            return false;
        };
        let offset = self.ui.scroll_offsets().get(id);
        let dy = layout.scroll_to(id, offset);
        if dy == 0 {
            return false;
        }
        let Some((region, view, content_h)) = layout
            .placed(id)
            .and_then(|p| p.scroll.map(|s| (p.rect, s.viewport, s.content_h)))
        else {
            return false;
        };
        // The strip the move left behind: at the bottom when the content
        // went up, at the top when it came back down.
        let strip = if dy > 0 {
            (view.bottom() - dy, view.bottom())
        } else {
            (view.y, view.y - dy)
        };
        // A shape the panel's own edge cuts is drawn a shade differently
        // from the whole of it, so a widget that crossed the edge before
        // the move or crosses it after is drawn again rather than moved.
        let edges = self.edge_rows(view, dy);
        let mut bands = [Some(strip), edges];
        bands.sort_by_key(|b| b.map_or(i32::MAX, |(y0, _)| y0));
        let tree = self.build();
        let (Some(canvas), Some(layout)) = (self.canvas.as_mut(), self.layout.as_ref()) else {
            return false;
        };
        canvas.shift(view, -dy);
        // The thumb was moved along with the pixels under it, so its
        // whole track is drawn again. The bands stop short of the track,
        // and of one another, so that no pixel is drawn over twice.
        // The scrollbar's thumb is drawn over the content and does not
        // move with it: where it was, now shifted along with the pixels,
        // and where it is are repainted, and nothing else of the column,
        // because the column crosses every row and repainting it would
        // repaint them all.
        let track = widgets::scrollbar_track(region, &ctx);
        let thumb_was = widgets::scrollbar_thumb(region, content_h, offset - dy, &ctx)
            .map(|t| Rect::new(t.x, t.y - dy, t.w, t.h));
        let thumb_now = widgets::scrollbar_thumb(region, content_h, offset, &ctx);
        let thumbs = match (thumb_was, thumb_now) {
            (Some(a), Some(b)) => {
                let y0 = a.y.min(b.y);
                let y1 = a.bottom().max(b.bottom());
                Some(Rect::new(track.x, y0, track.w, y1 - y0))
            }
            (a, b) => a.or(b),
        }
        .map(|t| t.intersect(&track).intersect(&view))
        .filter(|t| !t.is_empty());
        let mut drawn_to = view.y;
        for band in bands.into_iter().flatten() {
            let (y0, y1) = (band.0.max(drawn_to), band.1.min(view.bottom()));
            if y1 <= y0 {
                continue;
            }
            drawn_to = y1;
            let damage = Rect::new(view.x, y0, (track.x - view.x).max(0), y1 - y0);
            if damage.is_empty() {
                continue;
            }
            canvas.fill_rect(damage, self.theme.background);
            widgets::draw_tree_within(canvas, &tree, layout, &self.theme, &self.ui, damage);
        }
        if let Some(damage) = thumbs {
            canvas.fill_rect(damage, self.theme.background);
            widgets::draw_tree_within(canvas, &tree, layout, &self.theme, &self.ui, damage);
        }
        // The canvas now holds the frame of this layout, so nothing is
        // left to paint and the tree kept beside the layout is this one.
        self.tree = Some(tree);
        self.commands.push_back(Command::Draw);
        true
    }

    /// The rows of `view` where a widget crosses the panel's own edge,
    /// before a move of `dy` or after it: the pixels there cannot be
    /// moved, since a shape the edge cuts is drawn a shade differently
    /// from the whole of it.
    fn edge_rows(&self, view: Rect, dy: i32) -> Option<(i32, i32)> {
        let layout = self.layout.as_ref()?;
        let edge = self.canvas.as_ref()?.height();
        let crosses = |r: Rect| (r.y < 0 && r.bottom() > 0) || (r.y < edge && r.bottom() > edge);
        let mut rows: Option<(i32, i32)> = None;
        for p in &layout.items {
            // Only what paints: a container's rectangle is its children's
            // space, and a scroll region's content column always crosses
            // the panel's edge.
            if !p.draws || p.rect.is_empty() || p.rect.intersect(&view).is_empty() {
                continue;
            }
            let was = Rect::new(p.rect.x, p.rect.y + dy, p.rect.w, p.rect.h);
            if crosses(p.rect) || crosses(was) {
                let band = (p.rect.y.min(was.y), p.rect.bottom().max(was.bottom()));
                rows = Some(match rows {
                    Some((y0, y1)) => (y0.min(band.0), y1.max(band.1)),
                    None => band,
                });
            }
        }
        rows
    }

    /// Whether the frame on the panel is the frame of the state the app
    /// is in, so that a scroll can move its pixels instead of drawing
    /// them: the canvas holds the frame of the layout in hand, nothing is
    /// animating, no secret is counting down, and the event that scrolled
    /// did nothing else.
    fn pixels_can_move(&self) -> bool {
        !self.dirty
            && !self.selftest_failed()
            && !self.session.is_locked()
            && self.ui.scroll_only()
            && !self.ui.animating()
            && self.reveal_until.is_none()
            && self.mask_deadline.is_none()
            && self.screen != Screen::Sign
            && self.screen != Screen::Scan
            && !self.export.qr
    }

    /// Scrolls [`Self::scroll_into_view`] into its clip and redraws.
    /// `true` when it did, so the caller's own draw is the stale one and
    /// is dropped.
    fn scrolled_into_view(&mut self) -> bool {
        let Some((id, anchor)) = self.scroll_into_view.take() else {
            return false;
        };
        let Some(placed) = self.layout.as_ref().and_then(|l| l.placed(id)) else {
            return false;
        };
        let (rect, clip) = (placed.rect, placed.clip);
        let dy = match anchor {
            Anchor::Top => rect.y - clip.y,
            Anchor::InView if rect.bottom() > clip.bottom() => rect.bottom() - clip.bottom(),
            Anchor::InView if rect.y < clip.y => rect.y - clip.y,
            Anchor::InView => return false,
        };
        if dy == 0 {
            return false;
        }
        let offset = self.ui.scroll_offsets().get(ids::SCROLL);
        self.ui.set_scroll(ids::SCROLL, (offset + dy).max(0));
        self.render();
        true
    }

    // ----- FROST wallets (`docs/PLANNING.md` §16.103, §16.104 rule 3) --

    /// Whether the session is still waiting for the shell's entropy,
    /// which is what keeps a wizard's hold dead.
    pub(crate) fn entropy_pending(&self) -> bool {
        self.session.entropy_pending()
    }

    /// Whether anything secret is in memory.
    pub(crate) fn has_secrets(&self) -> bool {
        !self.keys.is_empty()
    }

    /// The words the Backup flow is over.
    pub(crate) fn backup_words<R>(
        &self,
        b: &BackupFlow,
        f: impl FnOnce(&Mnemonic) -> R,
    ) -> Option<R> {
        self.keys.get(b.key())?.mnemonic(self.session.key(), f)
    }

    /// What the backup's result names: the key's master fingerprint.
    pub(crate) fn backup_fingerprint(&self, b: &BackupFlow) -> String {
        self.keys.get(b.key()).map(|k| k.fingerprint).map_or_else(
            || String::from(self.strings().value_none),
            text::fingerprint_hex,
        )
    }

    /// The members of the FROST wallet at `w`, matched against the
    /// loaded keys by public share: what each is called — the
    /// fingerprint of its public share, which is the one printed beside
    /// the words — and the loaded key it is, where this device holds
    /// one.
    fn record_members(&self, w: usize) -> Vec<(String, Option<usize>)> {
        let s = self.strings();
        let Some(record) = self.wallets.get(w).and_then(|p| p.record()) else {
            return Vec::new();
        };
        (0..record.n())
            .map(|i| {
                let label = match record.share_fingerprint(i) {
                    Some(fp) => text::fingerprint_hex(fp),
                    None => String::from(s.wallet_origin_unknown),
                };
                let public = record.info.pubshares.get(i).copied().flatten();
                let loaded = public.and_then(|p| self.keys.iter().position(|k| k.share == Some(p)));
                (label, loaded)
            })
            .collect()
    }

    /// "Add this wallet" on a FROST review: the chosen keys are the
    /// group's first `t` shares, the rest are computed, and the words of
    /// each computed one follow.
    fn deal_group(&mut self) {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        if self.session.entropy_pending() {
            return;
        }
        let (n, t) = (w.n(), w.t());
        let chosen: Vec<usize> = w
            .keys()
            .iter()
            .filter_map(build::BuiltKey::loaded)
            .collect();
        if chosen.len() != t {
            return;
        }
        let members: Vec<&LoadedKey> = chosen.iter().filter_map(|i| self.keys.get(*i)).collect();
        if members.len() != t {
            return;
        }
        let network = self.network;
        let session_key = self.session.key();
        let mut acc: Vec<Chosen> = Vec::with_capacity(t);
        let mut dealt: Option<Dealer> = None;
        open_shares(&members, session_key, &mut acc, &mut |shares| {
            dealt = Dealer::deal(n, t, shares, network);
        });
        acc.clear();
        let Some(dealer) = dealt else {
            return;
        };
        self.with_build(|w| w.set_dealer(dealer));
    }

    /// Every tap on the dealt group's own screens: the words, the quiz
    /// and the record.
    fn tap_deal(&mut self, id: Id) {
        let now = self.now_ms;
        let class = self.class();
        let pane = self.words_pane_dp();
        let Some(Wizard::Build(w)) = &mut self.wizard else {
            return;
        };
        let step = w.step();
        let Some(d) = w.dealer_mut() else {
            return;
        };
        match step {
            build::Step::Words => {
                let count = d.words(d.show_at()).map_or(0, |m| m.indices().len());
                if id == ids::WORDS_PREV || id == ids::WORDS_NEXT {
                    let per_page = views::words::per_page(class, pane, Language::English);
                    let pages = count.div_ceil(per_page.max(1)).max(1);
                    d.page_by(id == ids::WORDS_PREV, pages);
                } else if id == ids::THRESHOLD_WORDS_CONTINUE {
                    w.go(build::Step::QuizStart);
                } else {
                    d.tap(id);
                }
            }
            build::Step::QuizStart => {
                if id == ids::QUIZ_HELPER {
                    d.toggle_helper();
                } else if id == ids::QUIZ_START {
                    if let Some(m) = d.words(d.show_at()) {
                        d.start_quiz(m.indices(), now);
                        w.go(build::Step::Quiz);
                    }
                } else if id == ids::QUIZ_SKIP {
                    w.go(build::Step::QuizSkip);
                }
            }
            build::Step::QuizSkip => {
                if id == ids::QUIZ_SKIP_CANCEL {
                    w.go(build::Step::QuizStart);
                } else if id == ids::QUIZ_SKIP_CONFIRM {
                    self.member_done();
                }
            }
            build::Step::Quiz => {
                let passed = match d.words(d.show_at()) {
                    Some(m) => d.quiz_tap(id, m.indices()),
                    None => false,
                };
                if passed {
                    self.member_done();
                }
            }
            build::Step::Record => {
                if id == ids::THRESHOLD_RECORD_CONTINUE {
                    self.finish_group();
                } else if id == ids::THRESHOLD_RECORD_SAVE && d.save() != sign::Save::Waiting {
                    self.save_record();
                }
            }
            _ => {}
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// One computed key is copied and quizzed: its words become a loaded
    /// key, and the next member's words or the record follows.
    fn member_done(&mut self) {
        let Some(Wizard::Build(w)) = &mut self.wizard else {
            return;
        };
        let Some(d) = w.dealer_mut() else {
            return;
        };
        let words = d.words(d.show_at());
        let more = d.next_member();
        w.go(if more {
            build::Step::Words
        } else {
            build::Step::Record
        });
        // Every key of the group this device made is loaded until it is
        // forgotten; Keys is where a person forgets the ones that belong
        // elsewhere (§16.104 rule 3).
        if let Some(m) = words {
            self.add_dealt_key(&m);
        }
        self.ui.set_scroll(ids::SCROLL, 0);
    }

    /// One dealt key, loaded the way a key read from words is.
    fn add_dealt_key(&mut self, m: &Mnemonic) {
        let network = self.network;
        let Some(key) = crate::finish::key_from_words(m, network) else {
            return;
        };
        if self.keys.iter().any(|k| k.fingerprint == key.fingerprint) {
            return;
        }
        let mut key = key;
        key.seal(self.session.key_mut());
        self.keys.push(key);
    }

    /// "Save" on the group record: the record's text through the shell,
    /// named by the wallet's fingerprint.
    fn save_record(&mut self) {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        let Some(record) = w.dealer().map(crate::threshold::Dealer::record) else {
            return;
        };
        let name_hint = strings::fill1(
            self.strings().threshold_file_name,
            &text::fingerprint_hex(record.fingerprint()),
        );
        let bytes = record.to_text().into_bytes();
        if let Some(Wizard::Build(w)) = &mut self.wizard
            && let Some(d) = w.dealer_mut()
        {
            d.mark_saving();
        }
        self.commands.push_back(Command::WriteFile {
            kind: FileKind::Any,
            name_hint,
            bytes,
        });
    }

    /// Continue on the group record: the record becomes a wallet in use
    /// and its page is where the wizard ends.
    fn finish_group(&mut self) {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return;
        };
        let Some(text) = w.dealer().map(|d| d.record().to_text()) else {
            return;
        };
        let Ok(policy) = WalletPolicy::parse_any(&text) else {
            return;
        };
        self.cancel_wizard();
        let wallet = match self.wallets.iter().position(|p| *p == policy) {
            Some(i) => i,
            None => {
                self.wallets.push(policy);
                self.sync_kept();
                self.wallets.len() - 1
            }
        };
        self.open_wallet(wallet);
    }
}

/// One member of a FROST group being dealt: its identifier and its
/// secret share, which exists only inside [`open_shares`].
type Chosen = (u32, SecShare);

/// Opens every member's secret share at once, each for the length of the
/// call the dealer runs inside: the scalars exist nowhere else, and the
/// recursion is what lets a value that is not `Clone` be collected.
fn open_shares(
    keys: &[&LoadedKey],
    session: &osk_crypto::SessionKey,
    acc: &mut Vec<Chosen>,
    f: &mut dyn FnMut(&[Chosen]),
) {
    match keys.first() {
        None => f(acc),
        Some(key) => {
            let rest = &keys[1..];
            key.secret_share(session, |secret| {
                let mut bytes = secret.secret_bytes();
                if let Ok(owned) = SecShare::from_bytes(&bytes) {
                    acc.push((acc.len() as u32, owned));
                    open_shares(rest, session, acc, f);
                }
                bytes.zeroize();
            });
        }
    }
}

/// The Explore secret the row `id` opens, if it is one of them
/// (`docs/DESIGN.md` §2.8: every long secret has a screen of its own).
/// Whether a payload is a secret: words in the clear, a seed code in
/// either form, an extended private key, or a private key in WIF.
///
/// This is what "a secret is never pasted or copied" (`docs/DESIGN.md`
/// §4.10) is enforced by: a payload that answers `true` is refused on
/// the way in from the clipboard, and refused under every calculator
/// whatever it arrived on.
/// The entropy behind a gathered run's words, which is the run's own
/// random bytes. The buffer is the caller's to zeroize.
fn entropy_bytes(m: &osk_bip::bip39::Mnemonic) -> ([u8; 32], usize) {
    let e = m.entropy();
    let mut buf = [0u8; 32];
    let n = e.expose().as_bytes().len().min(32);
    buf[..n].copy_from_slice(&e.expose().as_bytes()[..n]);
    (buf, n)
}

fn is_secret_payload(bytes: &[u8]) -> bool {
    if matches!(
        classify(bytes),
        PayloadKind::SeedQr { .. }
            | PayloadKind::CompactSeedQr { .. }
            | PayloadKind::Bip39Words { .. }
            | PayloadKind::Codex32
    ) {
        return true;
    }
    let Ok(text) = core::str::from_utf8(bytes.trim_ascii()) else {
        return false;
    };
    let text = text.trim();
    if osk_bip::xkey::decode_xpriv(text).is_ok() {
        return true;
    }
    // A WIF private key: base58check over 32 bytes, or 33 with the
    // compression flag, under the mainnet or test version byte.
    matches!(
        osk_codec::encodings::read(text),
        Some(r)
            if r.encoding == osk_codec::encodings::Encoding::Base58Check
                && matches!(r.version, Some(0x80 | 0xef))
                && (r.bytes.len() == 32 || r.bytes.len() == 33)
    )
}

/// The scanner a calculator opens on, where it takes a string.
fn expect_of(tool: Tool) -> Option<Expect> {
    match tool {
        Tool::Hashes => Some(Expect::Hashes),
        Tool::Encodings => Some(Expect::Encodings),
        Tool::Descriptor => Some(Expect::Descriptor),
        Tool::ConvertKey => Some(Expect::ConvertKey),
        Tool::Miniscript => Some(Expect::Policy),
        Tool::Units => None,
    }
}

fn explore_secret_of(id: Id) -> Option<explore::SecretKind> {
    match id {
        ids::EXPLORE_XPRV => Some(explore::SecretKind::Xprv),
        ids::EXPLORE_ENTROPY => Some(explore::SecretKind::Entropy),
        ids::EXPLORE_CHECKSUM_BITS => Some(explore::SecretKind::ChecksumBits),
        ids::EXPLORE_SEED => Some(explore::SecretKind::Seed),
        ids::EXPLORE_MASTER_XPRV => Some(explore::SecretKind::MasterXprv),
        _ => None,
    }
}

/// The address `xpub`'s public key pays to under `script` on `network`.
fn leaf_address(xpub: &Xpub, script: ScriptType, network: Network) -> String {
    use osk_bip::bitcoin::Address;
    let net = osk_bip::bitcoin::Network::from(network);
    let key = xpub.to_pub();
    let address = match script {
        ScriptType::Legacy => Address::p2pkh(key, net),
        ScriptType::NestedSegwit => Address::p2shwpkh(&key, net),
        ScriptType::NativeSegwit => Address::p2wpkh(&key, net),
        ScriptType::Taproot => Address::p2tr(
            &osk_bip::bitcoin::secp256k1::Secp256k1::verification_only(),
            xpub.to_x_only_pub(),
            None,
            net,
        ),
    };
    alloc::format!("{address}")
}

/// Whether the first QR widget in `node` draws its modules.
fn qr_visible_in(node: &Node, ui: &UiState) -> Option<bool> {
    if let Some(w @ Widget::Qr { .. }) = node.as_widget() {
        return Some(w.qr_visible(ui));
    }
    node.children.iter().find_map(|c| qr_visible_in(c, ui))
}

impl Drop for OpenSigner {
    /// Everything secret zeroizes here: see the module documentation for
    /// what rust-bitcoin cannot guarantee.
    fn drop(&mut self) {
        self.wizard = None;
        self.keys.clear();
        self.cache.clear();
        self.keep.zeroize();
        self.session.zeroize();
    }
}

impl App for OpenSigner {
    fn event(&mut self, event: Event) {
        match event {
            Event::Display(info) => {
                let info = DisplayInfo {
                    width: info.width.max(1),
                    height: info.height.max(1),
                    ..info
                };
                let scale = Scale::new(info.dpi);
                self.dpi = info.dpi;
                self.camera_fixed = info.camera_fixed;
                self.memory_mib = info.memory_mib;
                self.secure = info.secure;
                self.boot = info.boot;
                self.canvas = Some(Canvas::new(&info));
                self.ctx = Some(LayoutCtx::new(scale, SizeClass::of(&info)).with_insets(
                    f32::from(info.inset_top) / scale.factor(),
                    f32::from(info.inset_bottom) / scale.factor(),
                ));
                self.ui = UiState::new(scale);
                // §16.50: the class decides whether a candidate takes
                // one tap or two, so a display that changes class takes
                // the wizard's selection with it.
                let two_tap = self.class() == SizeClass::Small;
                match &mut self.wizard {
                    Some(Wizard::Load(w)) => w.set_two_tap(two_tap),
                    Some(Wizard::Create(w)) => w.set_two_tap(two_tap),
                    _ => {}
                }
                // What the app has before the shell's entropy: not a
                // substitute for it (`osk_crypto::SessionKey::weak`).
                let mut material = [0u8; 16];
                material[..2].copy_from_slice(&info.width.to_le_bytes());
                material[2..4].copy_from_slice(&info.height.to_le_bytes());
                material[4..6].copy_from_slice(&info.dpi.to_le_bytes());
                material[6] = info.buttons;
                material[8..].copy_from_slice(&self.now_ms.to_le_bytes());
                self.session.remix_weak(&material);
                self.request_entropy();
                if self.selftest.is_none() {
                    self.run_selftest();
                }
                // A Tier A board has no secure boot, so it says so on the
                // screen it opens on (security review 2026-09-11, M6).
                // Only at start: nothing has been drawn yet, and a
                // display that changes later is not a second start.
                if self.tier == AssuranceTier::A && self.layout.is_none() {
                    self.screen = Screen::NoSecureBoot;
                }
                self.render();
            }
            Event::Entropy(entropy) => {
                let mut bytes = entropy.into_bytes();
                // The Create wizard's device source asks for its own
                // answer; the session takes the same bytes only when it
                // had a request of its own outstanding.
                let wanted = self.create_entropy(&bytes);
                let rotated = self.entropy(&bytes) || wanted;
                bytes.zeroize();
                // The PIN pad scramble follows the key: redraw so that
                // the frame on screen and the hit targets agree.
                if rotated {
                    self.render();
                }
            }
            Event::Settings { bytes } => {
                self.apply_settings(&settings::read(&bytes));
                self.render();
            }
            Event::Lock => {
                self.lock();
                self.render();
            }
            Event::SecretKept { kept } => {
                self.secret_kept = kept;
                self.render();
            }
            Event::SecureMac { mac } => {
                self.secure_mac(&mac);
                self.render();
            }
            Event::SecureUnavailable => {
                self.keep_failed();
                self.render();
            }
            Event::Secret { blob } => {
                self.secret_blob(Some(blob));
                self.render();
            }
            Event::SecretUnavailable => {
                self.secret_blob(None);
                self.render();
            }
            Event::SecretStored => {
                self.secret_stored(true);
                self.render();
            }
            Event::SecretNotStored => {
                self.secret_stored(false);
                self.render();
            }
            Event::SecretForgotten => {
                self.secret_forgotten();
                self.render();
            }
            Event::File { bytes, .. } => self.file_event(Some(bytes)),
            Event::FileList {
                kind,
                entries,
                place,
            } => self.file_list_event(kind, entries, place),
            Event::FileUnavailable { .. } => self.file_event(None),
            Event::FileCancelled { .. } => self.file_cancelled(),
            Event::FileWritten { .. } => self.write_answer(true),
            Event::FileNotWritten { .. } => self.write_answer(false),
            Event::Clipboard { text, .. } => self.clipboard_event(Some(text)),
            Event::ClipboardUnavailable { .. } => self.clipboard_event(None),
            Event::ClipboardWritten { .. } => self.copy_answer(true),
            Event::ClipboardNotWritten { .. } => self.copy_answer(false),
            Event::CameraFrame {
                width,
                height,
                luma,
                chroma,
            } => self.camera_frame(width, height, luma, chroma),
            Event::Scanned { bytes } => self.scanned(bytes),
            Event::CameraUnavailable => self.camera_unavailable(),
            // Escape cancels the wizard outright (desktop accelerator);
            // elsewhere it is the back gesture, handled by `UiState`.
            Event::Key(Key::Escape) if self.wizard.is_some() && !self.selftest_failed() => {
                self.session.touched(self.now_ms);
                self.cancel_wizard();
                self.render();
            }
            Event::Tick { now_ms } => {
                self.now_ms = now_ms;
                let Some(layout) = self.layout.as_ref() else {
                    return;
                };
                let action = self.ui.event(layout, event);
                let mask = self.mask_deadline.is_some_and(|t| now_ms >= t);
                let leave_over = self.leave_deadline.is_some_and(|t| now_ms >= t);
                if leave_over {
                    self.leave_deadline = None;
                }
                // A grind is ticked by the shell's own frames: the
                // core has one thread and spends a budget of candidates
                // a frame (§16.117).
                let ground = self.vanity_tick();
                let animated = ground
                    | (self.screen == Screen::Sign
                        && self.wizard.is_none()
                        && self.sign.tick(now_ms))
                    | (self.export.qr
                        && self.export.run.as_mut().is_some_and(|run| run.tick(now_ms)));
                if let Some(a) = action {
                    self.apply(a);
                }
                if mask {
                    self.mask_deadline = None;
                }
                let fired = self.timers(now_ms);
                // The eye's countdown redraws once a second and the
                // panel masks itself when it runs out.
                let reveal = self.reveal_second != self.reveal_left_s();
                if action.is_some() || mask || leave_over || animated || fired || reveal {
                    self.render();
                }
            }
            other => {
                // Any touch, key, button or scroll restarts the timers.
                self.session.touched(self.now_ms);
                let Some(layout) = self.layout.as_ref() else {
                    return;
                };
                if let Some(a) = self.ui.event(layout, other) {
                    // A scroll shows the content that is already drawn,
                    // moved: the pixels move and only what came into
                    // view is drawn (PLANNING §16.36).
                    if let Action::Scrolled(id) = a
                        && self.scroll_moved(id)
                    {
                        return;
                    }
                    self.apply(a);
                    self.render();
                }
            }
        }
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.commands.pop_front()
    }

    fn frame(&mut self) -> Frame<'_> {
        // §16.119 rule 5: the paint happens here, from the tree and
        // layout the last event solved, and only when an event has
        // solved one since the canvas was last painted.
        if self.dirty {
            let Self {
                canvas,
                tree,
                layout,
                theme,
                ui,
                dirty,
                ..
            } = self;
            if let (Some(canvas), Some(tree), Some(layout)) =
                (canvas.as_mut(), tree.as_ref(), layout.as_ref())
            {
                canvas.clear(theme.background);
                widgets::draw_tree(canvas, tree, layout, theme, ui);
                *dirty = false;
            }
        }
        self.canvas.as_ref().map_or(
            Frame {
                width: 0,
                height: 0,
                rgba: &[],
            },
            Canvas::frame,
        )
    }
}

// ----- Notes, recovery sheets, the form an export takes, and the
// Argon2id memory an encrypted export is made at (`docs/PLANNING.md`
// §16.112) -----

impl OpenSigner {
    /// The Argon2id memory every `seal` uses, in KiB: what the person
    /// chose, or what this device recommends until they choose. The
    /// kept-key blob keeps its own 64 MiB, because that cost is paid on
    /// every unlock.
    pub(crate) fn backup_memory_kib(&self) -> u32 {
        self.backup_memory
            .unwrap_or_else(|| self.recommended_memory_kib())
    }

    /// The cost this device recommends: 256 MiB where the shell reports
    /// at least a gigabyte of memory, and the one that opens anywhere
    /// otherwise.
    pub(crate) fn recommended_memory_kib(&self) -> u32 {
        match self.memory_mib {
            Some(mib) if mib >= BACKUP_MEMORY_RECOMMENDS_MORE => BACKUP_MEMORY[1],
            _ => BACKUP_MEMORY[0],
        }
    }

    /// Tools › Notes.
    fn tap_notes(&mut self, id: ids::Id) {
        if id == ids::NOTES_NEW {
            self.note.clear();
            self.push(Screen::NoteText);
            return;
        }
        if id == ids::NOTES_READ_FILE {
            self.start_scan(Expect::Note);
            return;
        }
        if id == ids::at(ids::FORM_BASE, 0) && !self.note.is_empty() {
            self.push(Screen::Note);
            return;
        }
        // A kept note is taken in hand and shown on the same Document
        // the note just typed is.
        for i in 0..self.notes_kept.len() {
            if id != ids::at(ids::NOTES_KEPT_BASE, i) {
                continue;
            }
            let Some(note) = notes::Note::from_bytes(self.notes_kept[i].bytes()) else {
                return;
            };
            self.note.set(note);
            self.push(Screen::Note);
            return;
        }
        // A kept sheet with no wallet opens as an arrived sheet does,
        // which is the Document that offers "Add this wallet".
        for i in 0..self.sheets_kept.len() {
            if id != ids::at(ids::NOTES_SHEET_BASE, i) {
                continue;
            }
            let sheet = &self.sheets_kept[i];
            self.opened_sheet = Some(osk_backup::oskb::Sheet {
                descriptor: sheet.descriptor.clone(),
                name: sheet.name.clone(),
                note: sheet.note.clone(),
            });
            self.push(Screen::OpenedSheet);
            return;
        }
    }

    /// ✓ on the note entry: the note in hand, shown whole. A sheet's
    /// note goes back to the sheet it belongs to.
    fn note_typed(&mut self) {
        match self.stack.last().copied() {
            Some(Screen::Sheet(wallet)) => {
                let note = core::mem::take(&mut self.note);
                self.set_sheet_note(wallet, note);
                // A kept sheet carries the note it now has.
                if self.sheet_is_kept(wallet) {
                    self.sync_kept();
                }
                self.back();
            }
            _ => {
                self.screen = Screen::Note;
                self.ui.set_scroll(ids::SCROLL, 0);
            }
        }
    }

    /// The note document.
    fn tap_note(&mut self, id: ids::Id) {
        if id == ids::NOTE_EXPORT {
            self.ask_form(FormFor::Note);
        } else if id == ids::NOTE_KEEP {
            let kept = self.note_is_kept();
            self.set_note_kept(!kept);
        } else if id == ids::NOTE_FORGET {
            // Forgetting a note forgets it wherever it is: in hand and,
            // where it was kept, on the device (§16.112 pass E3).
            self.set_note_kept(false);
            self.note.clear();
            self.back();
        }
    }

    /// A wallet's recovery sheet.
    fn tap_sheet(&mut self, id: ids::Id, wallet: WalletRef) {
        if id == ids::SHEET_NOTE {
            let text = self.sheet_note_text(wallet);
            self.note.clear();
            if let Some(note) = notes::Note::from_bytes(text.as_bytes()) {
                self.note.set(note);
            }
            self.push(Screen::NoteText);
        } else if id == ids::SHEET_EXPORT {
            self.ask_form(FormFor::Sheet(wallet));
        } else if id == ids::SHEET_KEEP {
            let kept = self.sheet_is_kept(wallet);
            self.set_sheet_kept(wallet, !kept);
        } else if id == ids::SHEET_READ {
            // The same scanner Notes opens: a sealed sheet read here
            // opens as an arrived sheet, with "Add this wallet".
            self.start_scan(Expect::Note);
        }
    }

    /// A sheet that arrived encrypted: its descriptor is a wallet this
    /// device can register.
    fn tap_opened_sheet(&mut self, id: ids::Id) {
        if id != ids::SHEET_ADD_WALLET {
            return;
        }
        let Some(policy) = self.opened_sheet_policy() else {
            return;
        };
        let name = self
            .opened_sheet
            .as_ref()
            .map(|sheet| String::from_utf8_lossy(&sheet.name).into_owned())
            .unwrap_or_default();
        if !name.is_empty() {
            self.names.set_policy_name(&policy, &name);
        }
        let wallet = match self.wallets.iter().position(|w| *w == policy) {
            Some(i) => i,
            None => {
                self.wallets.push(policy);
                self.wallets.len() - 1
            }
        };
        // The sheet's note belongs to the wallet, so a kept sheet whose
        // wallet is added again is a wallet whose Recovery sheet reads
        // as it did (§16.112 pass E3).
        let note = self
            .opened_sheet
            .as_ref()
            .and_then(|sheet| notes::Note::from_bytes(&sheet.note));
        if let Some(note) = note
            && !note.is_empty()
        {
            self.set_sheet_note(WalletRef::Policy(wallet), note);
        }
        self.sync_kept();
        self.open_wallet(wallet);
    }

    /// The wallet an opened sheet's descriptor names, where it is one
    /// this device reads.
    pub(crate) fn opened_sheet_policy(&self) -> Option<WalletPolicy> {
        let sheet = self.opened_sheet.as_ref()?;
        let text = core::str::from_utf8(&sheet.descriptor).ok()?;
        WalletPolicy::parse_any(text.trim()).ok()
    }

    /// The descriptor a wallet's recovery sheet states.
    pub(crate) fn sheet_descriptor(&self, wallet: WalletRef) -> Option<String> {
        match wallet {
            WalletRef::Policy(i) => Some(self.wallets.get(i)?.to_descriptor_checksummed()),
            _ => None,
        }
    }

    /// The key a wallet's sheet note is kept under this session.
    fn sheet_key(&self, wallet: WalletRef) -> Option<String> {
        match wallet {
            WalletRef::Policy(i) => Some(self.wallets.get(i)?.checksum()),
            _ => None,
        }
    }

    /// The note written on `wallet`'s sheet, empty until one is.
    pub(crate) fn sheet_note_text(&self, wallet: WalletRef) -> String {
        let Some(key) = self.sheet_key(wallet) else {
            return String::new();
        };
        self.sheet_notes
            .iter()
            .find(|(k, _)| *k == key)
            .map_or_else(String::new, |(_, note)| String::from(note.text()))
    }

    /// Keeps `note` as `wallet`'s sheet note; an empty note takes it
    /// away.
    fn set_sheet_note(&mut self, wallet: WalletRef, note: notes::Note) {
        let Some(key) = self.sheet_key(wallet) else {
            return;
        };
        self.sheet_notes.retain(|(k, _)| *k != key);
        if !note.is_empty() {
            self.sheet_notes.push((key, note));
        }
    }

    /// "Which form?" for `what`, with the encrypted backup checked.
    fn ask_form(&mut self, what: FormFor) {
        self.form = Some((what, Form::Oskb));
        self.push(Screen::ExportForm);
    }

    /// Whether the export now being made is a plain file, which is not
    /// sealed and so has no encryption to state.
    pub(crate) fn form_is_plain(&self) -> bool {
        self.form.is_some_and(|(_, form)| form == Form::Plain)
    }

    /// Whether it was asked for as a KDBX 4 database. A KDBX file has
    /// no QR: it is kilobytes rather than hundreds of bytes, and the
    /// apps that read one read files.
    pub(crate) fn form_is_kdbx(&self) -> bool {
        self.form.is_some_and(|(_, form)| form == Form::Kdbx)
    }

    /// Bytes for one KDBX file's master seed, encryption IV, Argon2id
    /// salt and inner-stream key, derived from the session key and the
    /// count of exports made, so no two of a session draw the same.
    fn kdbx_seed(&mut self) -> [u8; osk_backup::kdbx::SEED_LEN] {
        self.backups_made = self.backups_made.wrapping_add(1);
        let mut seed = [0u8; osk_backup::kdbx::SEED_LEN];
        for (block, chunk) in seed.chunks_mut(64).enumerate() {
            let mut label = [0u8; 18];
            label[..13].copy_from_slice(b"osk-kdbx-seed");
            label[13..17].copy_from_slice(&self.backups_made.to_le_bytes());
            label[17] = block as u8;
            let derived = self.session.key().derive(&label);
            chunk.copy_from_slice(&derived.expose()[..chunk.len()]);
        }
        seed
    }

    /// The name a KDBX export is offered under: the same name every
    /// time, as an `osk-backup` file is (§16.96), so that a file on a
    /// card says nothing about what it holds. What the entry is called
    /// — the key's fingerprint, a note's first line, the wallet's name —
    /// is inside the file, where an heir who has the passphrase sees it.
    fn kdbx_file_name(&self, _what: FormFor) -> String {
        String::from(self.strings().kdbx_file_name)
    }

    /// The name the file a sealed note or sheet was written to is
    /// offered under: the form decides it.
    pub(crate) fn sealed_file_name(&self) -> String {
        match self.form {
            Some((what, Form::Kdbx)) => self.kdbx_file_name(what),
            _ => String::from(self.strings().seal_file_name),
        }
    }

    /// The KDBX 4 database a note makes: its first line as the Title
    /// and the note itself as the Notes, which is where a KeePass app
    /// shows text.
    fn kdbx_note(
        &self,
        passphrase: &[u8],
        cost: osk_backup::Cost,
        seed: &[u8; osk_backup::kdbx::SEED_LEN],
    ) -> Option<Vec<u8>> {
        let s = self.strings();
        let title = note_title(self.note.text(), s.kdbx_note_title);
        osk_backup::kdbx::write(
            &[osk_backup::kdbx::Item {
                title: title.as_bytes(),
                notes: self.note.bytes(),
                secret: &[],
            }],
            passphrase,
            cost,
            seed,
        )
    }

    /// The KDBX 4 database a wallet's recovery sheet makes: the name as
    /// the Title, the descriptor and the sheet's note as the Notes.
    fn kdbx_sheet(
        &self,
        wallet: WalletRef,
        passphrase: &[u8],
        cost: osk_backup::Cost,
        seed: &[u8; osk_backup::kdbx::SEED_LEN],
    ) -> Option<Vec<u8>> {
        let s = self.strings();
        let title = self.wallet_name(wallet).unwrap_or_default();
        let notes = alloc::format!(
            "{}\n{}\n\n{}\n{}",
            s.sheet_plain_descriptor,
            self.sheet_descriptor(wallet).unwrap_or_default(),
            s.sheet_plain_note,
            self.sheet_note_text(wallet),
        );
        osk_backup::kdbx::write(
            &[osk_backup::kdbx::Item {
                title: title.as_bytes(),
                notes: notes.as_bytes(),
                secret: &[],
            }],
            passphrase,
            cost,
            seed,
        )
    }

    /// Whether the sealed note or sheet fits one QR, which is where its
    /// Result offers "Show as QR".
    pub(crate) fn sealed_fits_qr(&self) -> bool {
        self.sealed.len() <= MAX_QR_BYTES
    }

    /// The form Choice.
    fn tap_export_form(&mut self, id: ids::Id) {
        let Some((what, _)) = self.form else {
            return;
        };
        if let Some(i) = ids::index_in(id, ids::FORM_BASE, Form::ALL.len()) {
            self.form = Some((what, Form::ALL[i]));
            return;
        }
        if id != ids::FORM_CONTINUE {
            return;
        }
        let Some((what, form)) = self.form else {
            return;
        };
        match (what, form) {
            // A plain export is written at once: there is no passphrase
            // to ask for.
            (FormFor::Note, Form::Plain) => {
                let bytes = self.note.bytes().to_vec();
                let name = String::from(self.strings().note_file_name);
                self.write_plain(bytes, name);
            }
            (FormFor::Sheet(wallet), Form::Plain) => {
                let bytes = self.sheet_text(wallet).into_bytes();
                let name = String::from(self.strings().sheet_file_name);
                self.write_plain(bytes, name);
            }
            // Everything else is sealed, so the passphrase comes next.
            (FormFor::Backup(key), _) => {
                self.pop_screen();
                self.start_backup(key, BackupStep::Passphrase);
            }
            _ => {
                self.seal_pass.zeroize();
                self.sealed.clear();
                self.sealed_save = Save::Idle;
                self.screen = Screen::SealPass;
                self.ui.set_scroll(ids::SCROLL, 0);
            }
        }
    }

    /// A recovery sheet as the plain text file it is exported as: three
    /// labelled sections, in the order the Document states them.
    fn sheet_text(&self, wallet: WalletRef) -> String {
        let s = self.strings();
        alloc::format!(
            "{}\n{}\n\n{}\n{}\n\n{}\n{}\n",
            s.sheet_plain_descriptor,
            self.sheet_descriptor(wallet).unwrap_or_default(),
            s.sheet_plain_name,
            self.wallet_name(wallet).unwrap_or_default(),
            s.sheet_plain_note,
            self.sheet_note_text(wallet),
        )
    }

    /// Hands the shell a plain file and lands on the Result that says
    /// what became of it.
    fn write_plain(&mut self, bytes: Vec<u8>, name: String) {
        self.sealed = bytes;
        self.sealed_save = Save::Waiting;
        self.screen = Screen::Sealed;
        self.ui.set_scroll(ids::SCROLL, 0);
        let bytes = self.sealed.clone();
        self.commands.push_back(Command::WriteFile {
            kind: FileKind::Any,
            name_hint: name,
            bytes,
        });
    }

    /// The repeat matched: seal what the form was asked for.
    fn seal_now(&mut self) {
        let cost = self.export_cost();
        let Some((what, form)) = self.form else {
            return;
        };
        // A KDBX database is written from the same plaintext, under the
        // same passphrase and at the same cost; what changes is the
        // container an heir opens it in (§16.112 pass E2).
        if form == Form::Kdbx {
            let seed = self.kdbx_seed();
            let made = match what {
                FormFor::Note => self.kdbx_note(self.seal_pass.expose(), cost, &seed),
                FormFor::Sheet(wallet) => {
                    self.kdbx_sheet(wallet, self.seal_pass.expose(), cost, &seed)
                }
                FormFor::Backup(_) => None,
            };
            self.seal_pass.zeroize();
            self.mask_deadline = None;
            self.sealed_made(made);
            return;
        }
        let seed = self.backup_seed();
        let sealed = match (what, form) {
            (FormFor::Note, _) => {
                osk_backup::oskb::seal_note(self.note.bytes(), self.seal_pass.expose(), cost, &seed)
            }
            (FormFor::Sheet(wallet), _) => osk_backup::oskb::seal_sheet(
                self.sheet_descriptor(wallet).unwrap_or_default().as_bytes(),
                self.wallet_name(wallet).unwrap_or_default().as_bytes(),
                self.sheet_note_text(wallet).as_bytes(),
                self.seal_pass.expose(),
                cost,
                &seed,
            ),
            (FormFor::Backup(_), _) => None,
        };
        self.seal_pass.zeroize();
        self.mask_deadline = None;
        self.sealed_made(sealed);
    }

    /// What became of a seal: the Result, or back where nothing was
    /// made.
    fn sealed_made(&mut self, sealed: Option<Vec<u8>>) {
        match sealed {
            Some(bytes) => {
                self.sealed = bytes;
                self.sealed_save = Save::Idle;
                self.screen = Screen::Sealed;
                self.ui.set_scroll(ids::SCROLL, 0);
            }
            None => self.back(),
        }
    }

    /// The sealed file's Result and its QR.
    fn tap_sealed(&mut self, id: ids::Id) {
        if id == ids::SEAL_SAVE {
            self.sealed_save = Save::Waiting;
            let bytes = self.sealed.clone();
            let name = self.sealed_file_name();
            self.commands.push_back(Command::WriteFile {
                kind: FileKind::Any,
                name_hint: name,
                bytes,
            });
        } else if id == ids::SEAL_SHOW_QR {
            self.screen = Screen::SealedQr;
            self.ui.set_scroll(ids::SCROLL, 0);
        } else if id == ids::SEAL_DONE {
            self.go_home();
        }
    }

    /// A note that arrived: read from a plain file, or opened out of an
    /// encrypted one. It lands on the Document that shows it whole.
    fn note_opened(&mut self, bytes: &[u8]) {
        match notes::Note::from_bytes(bytes) {
            Some(note) => {
                self.note.set(note);
                self.go_home();
                self.push(Screen::Tools);
                self.push(Screen::Notes);
                self.push(Screen::Note);
            }
            None => {
                let reason = String::from(self.strings().note_not_text);
                self.scan_failed(reason);
            }
        }
    }

    /// A recovery sheet that arrived encrypted and opened.
    fn sheet_opened(&mut self, sheet: &osk_backup::oskb::Sheet) {
        self.opened_sheet = Some(osk_backup::oskb::Sheet {
            descriptor: sheet.descriptor.clone(),
            name: sheet.name.clone(),
            note: sheet.note.clone(),
        });
        self.go_home();
        self.push(Screen::OpenedSheet);
    }
}

/// A note's first line, which is what a KDBX entry is titled by. The
/// note itself carries no title — nothing on the device asks for one —
/// so the line a person wrote first is what names it, and a note that
/// starts blank falls back to `default`.
fn note_title<'a>(text: &'a str, default: &'static str) -> &'a str {
    let line = text.lines().next().unwrap_or_default().trim();
    if line.is_empty() { default } else { line }
}

/// The pixels on each side of one module of a code saved as a PNG: a
/// version-40 code is then 1480 pixels square with its quiet zone.
const PNG_MODULE_PX: usize = 8;

/// Bytes one QR holds at the error correction the app writes, which is
/// where a sealed file's Result offers "Show as QR".
const MAX_QR_BYTES: usize = 1200;

// ---------------------------------------------------------------------
// Silent payments (`docs/PLANNING.md` §16.113)
// ---------------------------------------------------------------------

impl OpenSigner {
    /// The record of the wallet at `wallet`, where it is a silent
    /// payments wallet.
    pub(crate) fn silent_of(&self, wallet: usize) -> Option<&osk_bip::silent_wallet::SilentWallet> {
        self.wallets.get(wallet)?.silent()
    }

    /// Whether `wallet` is one.
    pub(crate) fn is_silent(&self, wallet: WalletRef) -> bool {
        matches!(wallet, WalletRef::Policy(w) if self.silent_of(w).is_some())
    }

    /// The scan and spend keys of that wallet, derived from the loaded
    /// key it was built on. `None` where that key is not loaded, which
    /// is what leaves the labels and the check without an answer.
    fn silent_receiver(&self, wallet: usize) -> Option<(usize, osk_bip::silent::Receiver)> {
        let record = self.silent_of(wallet)?;
        let (i, key) =
            self.keys.iter().enumerate().find(|(_, k)| {
                k.fingerprint == record.fingerprint && k.network == record.network
            })?;
        let master = key.master.as_ref()?;
        let receiver = osk_bip::silent::Receiver::derive(master, record.account);
        // The key must be the one the record was written from: a
        // fingerprint is four bytes, and the wallet is the two points.
        if receiver.spend_public_key() != record.spend {
            return None;
        }
        Some((i, receiver))
    }

    /// The address of that wallet, or of one of its labels. A labelled
    /// address needs the scan private key, so it exists only while the
    /// wallet's key is loaded.
    pub(crate) fn silent_address_text(&self, wallet: usize, label: Option<u32>) -> Option<String> {
        let record = self.silent_of(wallet)?;
        let Some(m) = label else {
            return Some(record.address());
        };
        let (i, receiver) = self.silent_receiver(wallet)?;
        let secp = self.keys.get(i)?.master.as_ref()?.secp();
        let text = receiver.labelled_address(secp, m).ok()?;
        Some(String::from(text.as_str()))
    }

    /// That address in the form the Address screen is showing it in.
    pub(crate) fn silent_shown(&self, wallet: usize, label: Option<u32>) -> Option<String> {
        let address = self.silent_address_text(wallet, label)?;
        match self.silent_form {
            silent::AddressForm::Address => Some(address),
            silent::AddressForm::Uri => osk_bip::silent::uri(&address)
                .ok()
                .map(|u| String::from(u.as_str())),
        }
    }

    /// Hands out the next label, which is a row of the Labels list and
    /// a line of the kept record.
    fn silent_hand_out_label(&mut self, wallet: usize) {
        let Some(record) = self.silent_of(wallet) else {
            return;
        };
        if record.labels >= osk_bip::silent_wallet::MAX_LABELS {
            return;
        }
        let mut next = record.clone();
        next.labels += 1;
        let name = self
            .names
            .policy_name(&self.wallets[wallet])
            .map(String::from);
        let policy = osk_bip::policy::WalletPolicy::of_silent(next);
        if let Some(name) = name {
            self.names.set_policy_name(&policy, &name);
        }
        self.wallets[wallet] = policy;
        self.sync_kept();
    }

    /// The wallet the Add a wallet review describes, once its one key
    /// is chosen: the two public keys BIP-352 derives, the origin of
    /// the key they came from, and no labels yet.
    pub(crate) fn silent_built(&self) -> Option<osk_bip::silent_wallet::SilentWallet> {
        let Some(Wizard::Build(w)) = &self.wizard else {
            return None;
        };
        if w.kind() != build::WalletKind::Silent {
            return None;
        }
        let which = w.keys().first().and_then(build::BuiltKey::loaded)?;
        let key = self.keys.get(which)?;
        let master = key.master.as_ref()?;
        let receiver = osk_bip::silent::Receiver::derive(master, 0);
        Some(osk_bip::silent_wallet::SilentWallet {
            network: key.network,
            fingerprint: key.fingerprint,
            account: 0,
            scan: receiver.scan_public_key(master.secp()),
            spend: receiver.spend_public_key(),
            labels: 0,
        })
    }

    /// Reads a transaction into the check, or, where one is already
    /// being checked, the previous transaction it is waiting for.
    fn silent_scanned(&mut self, bytes: &[u8], previous: bool) {
        // The scanner stands over the check, so the wallet is the one
        // on the stack under it.
        let Some(wallet) = core::iter::once(&self.screen)
            .chain(self.stack.iter().rev())
            .find_map(|s| match s {
                Screen::SilentCheck(w) => Some(*w),
                _ => None,
            })
        else {
            return;
        };
        if previous {
            let taken = self
                .check_payment
                .as_mut()
                .is_some_and(|c| c.add_previous(bytes));
            if !taken {
                let reason = String::from(self.strings().sign_not_psbt);
                self.scan_failed(reason);
                return;
            }
        } else {
            match silent::Check::read(bytes) {
                Some(check) => self.check_payment = Some(check),
                None => {
                    let reason = String::from(self.strings().sign_not_psbt);
                    self.scan_failed(reason);
                    return;
                }
            }
        }
        self.back();
        self.run_silent_check(wallet);
    }

    /// Works the answer out, once every previous output is known.
    fn run_silent_check(&mut self, wallet: usize) {
        let result = self.silent_answer(wallet);
        if let (Some(result), Some(check)) = (result, self.check_payment.as_mut()) {
            check.set_result(result);
        }
    }

    /// That answer, or `None` while a previous transaction is still
    /// missing.
    fn silent_answer(&self, wallet: usize) -> Option<silent::Checked> {
        let check = self.check_payment.as_ref()?;
        let prevouts = check.prevouts()?;
        let Some(record) = self.silent_of(wallet) else {
            return Some(silent::Checked::KeyNotLoaded);
        };
        let tx = check.tx();
        // An input whose key is in its signature data, with no
        // signature data, is a transaction that has not been signed:
        // reading it as one with fewer inputs would give a wrong answer
        // rather than no answer.
        for (i, input) in tx.input.iter().enumerate() {
            if osk_bip::silent::needs_signature_data(prevouts[i].as_bytes())
                && !osk_psbt::transaction::is_signed(&input.script_sig, &input.witness)
            {
                return Some(silent::Checked::Unsigned);
            }
        }
        let Some((i, receiver)) = self.silent_receiver(wallet) else {
            return Some(silent::Checked::KeyNotLoaded);
        };
        let secp = self.keys.get(i)?.master.as_ref()?.secp();
        // The change label is always scanned for, as BIP-352 asks.
        let labels: Vec<u32> = (0..=record.labels).collect();
        let scripts: Vec<&[u8]> = prevouts.iter().map(|s| s.as_bytes()).collect();
        match osk_bip::silent::find_payments(secp, &receiver, tx, &scripts, &labels) {
            Err(_) => Some(silent::Checked::NoInputs),
            Ok(found) if found.is_empty() => Some(silent::Checked::NotPaid),
            Ok(found) => Some(silent::Checked::Paid(
                found
                    .into_iter()
                    .map(|p| silent::Paid {
                        vout: p.vout,
                        amount: p.amount,
                        label: p.label,
                    })
                    .collect(),
            )),
        }
    }

    /// BIP-392's `sp([origin]spscan1q…)` for that wallet, which the
    /// Secret screen draws and nothing keeps.
    pub(crate) fn silent_descriptor(&self, wallet: usize) -> Option<String> {
        let record = self.silent_of(wallet)?;
        let (_, receiver) = self.silent_receiver(wallet)?;
        let text = receiver
            .descriptor(Some((record.fingerprint, record.account)))
            .ok()?;
        Some(String::from(text.as_str()))
    }

    /// A silent payments wallet's own screens.
    fn tap_silent_address(&mut self, id: ids::Id) {
        if id == ids::SILENT_FORM {
            let picked = silent::AddressForm::ALL
                .iter()
                .position(|f| *f == self.silent_form)
                .unwrap_or(0);
            self.overlay = Some(Overlay::Choice(Picker::SilentForm, picked));
        }
    }

    /// The Labels list.
    fn tap_silent_labels(&mut self, id: ids::Id, wallet: usize) {
        let labels = self.silent_of(wallet).map_or(0, |r| r.labels);
        if id == ids::SILENT_ADD_LABEL {
            self.silent_hand_out_label(wallet);
            return;
        }
        if let Some(i) = ids::index_in(id, ids::SILENT_LABEL_BASE, labels as usize) {
            self.push(Screen::SilentAddress(wallet, Some(i as u32 + 1)));
        }
    }

    /// Check a payment.
    fn tap_silent_check(&mut self, id: ids::Id) {
        if id == ids::SILENT_READ {
            self.check_payment = None;
            self.start_scan(Expect::SilentPayment);
        } else if id == ids::SILENT_PREVIOUS {
            self.start_scan(Expect::SilentPrevious);
        } else if id == ids::SILENT_DONE {
            self.check_payment = None;
            self.back();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use osk_shell_api::TouchPhase;

    fn display() -> Event {
        Event::Display(DisplayInfo {
            width: 640,
            height: 480,
            dpi: 286,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        })
    }

    #[test]
    fn selftest_runs_before_the_first_screen_and_passes() {
        let mut app = OpenSigner::new(
            AssuranceTier::C,
            BuildInfo {
                version: "test",
                core_hash: None,
            },
        );
        assert_eq!(app.selftest(), None);
        app.event(display());
        assert_eq!(app.selftest(), Some(Ok(osk_selftest::CHECKS.len())));
        // A device that has never run opens on the first run's
        // document; a device that has settings opens on Home.
        assert_eq!(app.screen(), ScreenKind::StartHere);
    }

    #[test]
    fn a_failing_check_blocks_everything_but_exit() {
        let mut app = OpenSigner::new(
            AssuranceTier::C,
            BuildInfo {
                version: "test",
                core_hash: None,
            },
        );
        app.fail_selftest = true;
        app.event(display());
        assert_eq!(app.poll_command(), Some(Command::RequestEntropy));
        assert_eq!(app.poll_command(), Some(Command::Draw));
        assert_eq!(app.selftest(), Some(Err("injected failure")));
        assert_eq!(app.screen(), ScreenKind::SelfTestFailed);
        assert!(
            app.rect_of(ids::at(ids::HOME_TILE_BASE, 0)).is_none(),
            "Home is not drawn"
        );
        assert!(app.rect_of(ids::BACK).is_none(), "no navigation");
        // §5 Result: the failure and the check that did not reproduce,
        // built from the screens module like every other screen.
        assert!(app.rect_of(ids::SCREEN).is_some(), "built from §5");
        let texts = app.texts();
        for want in [
            EN.selftest_title,
            EN.selftest_failed_title,
            EN.selftest_check_row,
            EN.selftest_exit,
            "injected failure",
        ] {
            assert!(texts.iter().any(|t| t == want), "{want:?}: {texts:?}");
        }
        // Back and Escape do nothing.
        app.event(Event::Button(osk_shell_api::ButtonId::Back));
        app.event(Event::Key(Key::Escape));
        while app.poll_command().is_some() {}
        assert_eq!(app.screen(), ScreenKind::SelfTestFailed);
        // Exit is the only control.
        let r = app.rect_of(ids::SELFTEST_EXIT).expect("exit button");
        let c = r.center();
        let (x, y) = (c.x as u16, c.y as u16);
        app.event(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        app.event(Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        });
        let mut commands = Vec::new();
        while let Some(c) = app.poll_command() {
            commands.push(c);
        }
        assert!(commands.contains(&Command::Exit), "{commands:?}");
    }
}
