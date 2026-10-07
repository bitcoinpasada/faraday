//! Which Learn page a screen is about (`docs/PLANNING.md` §16.105).
//!
//! The app bar's trailing slot holds the eye on a secret screen;
//! otherwise it holds the info button on any screen this table has a
//! page for, and otherwise it is empty. The button opens that page, at
//! one of its sections where the table names one, and the chevron comes
//! back to the screen it was tapped on.
//!
//! One table, both directions: [`Topic`] names a Learn page, the map
//! below says which topic a screen is about, and [`Topic::try_it`] says
//! which flow a page's "Try it" row opens (§16.85). A page's index in
//! [`Strings::learn_pages`] is looked up from the page itself, so
//! neither direction can drift from the order Learn lists them in.

use osk_ui::widgets::Icon;

use crate::backup::BackupStep;
use crate::build::{Step as BuildStep, WalletKind};
use crate::create::Step as CreateStep;
use crate::load::{Source as LoadSource, Step as LoadStep};
use crate::sign::{Stage, Step as SignStep};
use crate::strings::{LearnPage, Strings};
use crate::{OpenSigner, Screen, TryIt, WalletRef, Wizard};

use osk_bip::policy::Template;

/// A Learn page, named once so that both directions of the map read the
/// same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Topic {
    StartHere,
    Words,
    Devices,
    Transactions,
    Randomness,
    WhereRandomness,
    Backups,
    EncryptedBackups,
    SeedXor,
    Passphrases,
    Verifying,
    AirGap,
    Scams,
    Multisig,
    SpendPaths,
    Xpubs,
    SecureElement,
    Inheritance,
    Nonces,
    Message,
    Glossary,
    Tools,
    WalletKinds,
    Frost,
    OtherBackups,
    Coordinators,
    SilentPayments,
}

impl Topic {
    /// Every topic. Order does not matter: a topic finds its page by
    /// identity, not by its place here.
    const ALL: [Topic; 27] = [
        Topic::StartHere,
        Topic::Words,
        Topic::Devices,
        Topic::Transactions,
        Topic::Randomness,
        Topic::WhereRandomness,
        Topic::Backups,
        Topic::EncryptedBackups,
        Topic::SeedXor,
        Topic::Passphrases,
        Topic::Verifying,
        Topic::AirGap,
        Topic::Scams,
        Topic::Multisig,
        Topic::SpendPaths,
        Topic::Xpubs,
        Topic::SecureElement,
        Topic::Inheritance,
        Topic::Nonces,
        Topic::Message,
        Topic::Glossary,
        Topic::Tools,
        Topic::WalletKinds,
        Topic::Frost,
        Topic::OtherBackups,
        Topic::Coordinators,
        Topic::SilentPayments,
    ];

    /// The page itself.
    pub(crate) fn page(self, s: &'static Strings) -> &'static LearnPage {
        match self {
            Topic::StartHere => &s.learn_start_here,
            Topic::Words => &s.learn_words,
            Topic::Devices => &s.learn_devices,
            Topic::Transactions => &s.learn_transactions,
            Topic::Randomness => &s.learn_randomness,
            Topic::WhereRandomness => &s.learn_where_randomness,
            Topic::Backups => &s.learn_backups,
            Topic::EncryptedBackups => &s.learn_encrypted_backups,
            Topic::SeedXor => &s.learn_seed_xor,
            Topic::Passphrases => &s.learn_passphrases,
            Topic::Verifying => &s.learn_verifying,
            Topic::AirGap => &s.learn_air_gap,
            Topic::Scams => &s.learn_scams,
            Topic::Multisig => &s.learn_multisig,
            Topic::SpendPaths => &s.learn_spend_paths,
            Topic::Xpubs => &s.learn_xpubs,
            Topic::SecureElement => &s.learn_secure_element,
            Topic::Inheritance => &s.learn_inheritance,
            Topic::Nonces => &s.learn_nonces,
            Topic::Message => &s.learn_message,
            Topic::Glossary => &s.learn_glossary,
            Topic::Tools => &s.learn_tools,
            Topic::WalletKinds => &s.learn_wallet_kinds,
            Topic::Frost => &s.learn_frost,
            Topic::OtherBackups => &s.learn_other_backups,
            Topic::Coordinators => &s.learn_coordinators,
            Topic::SilentPayments => &s.learn_silent_payments,
        }
    }

    /// Its place in the list Learn shows, which is what a screen is
    /// pushed with.
    pub(crate) fn index(self, s: &'static Strings) -> usize {
        let page = self.page(s);
        s.learn_pages()
            .iter()
            .position(|p| core::ptr::eq(*p, page))
            .unwrap_or(0)
    }

    /// The topic page `i` of that list is.
    pub(crate) fn of(s: &'static Strings, i: usize) -> Option<Topic> {
        Topic::ALL.into_iter().find(|t| t.index(s) == i)
    }

    /// The last row of the page, when its subject is one flow: what the
    /// row starts, its label and its mark (§16.85). A page opened from a
    /// working screen's info button draws no such row, because the
    /// person is already in the flow.
    pub(crate) fn try_it(self, s: &'static Strings) -> Option<(TryIt, &'static str, Icon)> {
        match self {
            Topic::Words => Some((TryIt::CreateKey, s.home_create_key, Icon::Dice)),
            Topic::Randomness => Some((TryIt::RollDice, s.learn_try_dice, Icon::Dice)),
            Topic::Backups => Some((TryIt::VerifyBackup, s.learn_try_verify, Icon::Checklist)),
            Topic::EncryptedBackups => {
                Some((TryIt::EncryptedBackup, s.learn_try_encrypted, Icon::Vault))
            }
            Topic::Passphrases => Some((
                TryIt::OpenPassphrase,
                s.learn_try_passphrase,
                Icon::Passphrase,
            )),
            Topic::Verifying => Some((TryIt::CheckAddress, s.wallet_check, Icon::Verify)),
            Topic::Multisig => Some((TryIt::NewWallet, s.build_new, Icon::LayerGroup)),
            Topic::SpendPaths => Some((TryIt::Miniscript, s.learn_try_miniscript, Icon::Tools)),
            Topic::Xpubs => Some((TryIt::Export, s.wallet_export, Icon::Export)),
            Topic::Message => Some((TryIt::SignMessage, s.wallet_sign_message, Icon::Envelope)),
            _ => None,
        }
    }
}

/// Which section of "Kinds of wallets" a kind of wallet is, by its place
/// among that page's sections.
mod kinds {
    pub(super) const SINGLE: usize = 1;
    pub(super) const MULTISIG: usize = 2;
    pub(super) const TAPROOT_MULTISIG: usize = 3;
    pub(super) const MUSIG: usize = 4;
    pub(super) const FROST: usize = 5;
    pub(super) const RECOVERY: usize = 6;
    pub(super) const SILENT: usize = 7;
}

/// Which section of "Backups in other forms" a form of backup is, by its
/// place among that page's sections.
mod other_backups {
    pub(super) const SLIP39: usize = 1;
    pub(super) const CODEX32: usize = 2;
}

/// What the info button opens: a page, and the section it starts at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LearnTarget {
    /// The page.
    pub topic: Topic,
    /// The section to scroll to, by its place in the page's own
    /// sections. `None` opens the page at the top.
    pub section: Option<usize>,
}

/// A page with no section named.
fn page(topic: Topic) -> Option<LearnTarget> {
    Some(LearnTarget {
        topic,
        section: None,
    })
}

/// A page at one of its sections.
fn section(topic: Topic, section: usize) -> Option<LearnTarget> {
    Some(LearnTarget {
        topic,
        section: Some(section),
    })
}

/// The section of "Kinds of wallets" that a wallet kind being built is.
fn kind_section(kind: WalletKind) -> usize {
    match kind {
        WalletKind::Single => kinds::SINGLE,
        WalletKind::Multisig => kinds::MULTISIG,
        WalletKind::TaprootMultisig => kinds::TAPROOT_MULTISIG,
        WalletKind::MuSig2 => kinds::MUSIG,
        WalletKind::Frost => kinds::FROST,
        WalletKind::Recovery => kinds::RECOVERY,
        WalletKind::Silent => kinds::SILENT,
    }
}

impl OpenSigner {
    /// The Learn page the screen now drawn is about, or `None` where it
    /// has none and the app bar's trailing slot stays empty.
    ///
    /// Read in the order the screens are drawn in: an overlay and the
    /// scanner over a wizard first, then the wizard, then the screen
    /// underneath. Home, Learn itself, the lock screen, the PIN pads,
    /// the Words, Secret and Compare screens and the holds that forget
    /// and wipe are in none of the arms, which is what leaves the slot
    /// empty on them.
    pub(crate) fn learn_target(&self) -> Option<LearnTarget> {
        if self.session.is_locked() || self.front_door() || self.overlay.is_some() {
            return None;
        }
        if matches!(self.screen, Screen::LearnTopic(..)) {
            return None;
        }
        if self.building_scan() {
            return page(Topic::AirGap);
        }
        if let Some(w) = &self.wizard {
            return self.wizard_target(w);
        }
        self.screen_target()
    }

    /// The page a wizard step is about.
    fn wizard_target(&self, w: &Wizard) -> Option<LearnTarget> {
        match w {
            // Create SLIP-39 shares: the word count a share has, and
            // every screen of the plan, are about the form the backup
            // is written in (§16.107 rule 4).
            Wizard::Create(w) if w.is_slip39() && w.step() == CreateStep::Count => {
                section(Topic::OtherBackups, other_backups::SLIP39)
            }
            Wizard::Create(w) if w.step() == CreateStep::Shares => {
                section(Topic::OtherBackups, other_backups::SLIP39)
            }
            // Create Codex32 shares: the seed length and every screen of
            // the plan are about the form the backup is written in
            // (§16.109 rule 4).
            Wizard::Create(w) if w.is_codex32() && w.step() == CreateStep::Count => {
                section(Topic::OtherBackups, other_backups::CODEX32)
            }
            Wizard::Create(w) if w.step() == CreateStep::Codex32 => {
                section(Topic::OtherBackups, other_backups::CODEX32)
            }
            Wizard::Create(w) => match w.step() {
                CreateStep::Source => page(Topic::Randomness),
                CreateStep::Procedure
                | CreateStep::Entropy
                | CreateStep::LastWord
                | CreateStep::Camera
                | CreateStep::Device
                | CreateStep::MixChoose
                | CreateStep::MixResult => page(Topic::WhereRandomness),
                CreateStep::PassphraseOffer
                | CreateStep::Passphrase
                | CreateStep::PassphraseConfirm => page(Topic::Passphrases),
                _ => None,
            },
            Wizard::Load(w) => match w.step() {
                // The source Choice is the step that says what is being
                // loaded, so its page is the page of the checked row.
                LoadStep::Source => match w.source() {
                    // Numbers are words by another notation, and hex
                    // entropy is what the words encode, so both rows
                    // land on the words page (§16.125 rule 4).
                    LoadSource::Type
                    | LoadSource::SeedCode
                    | LoadSource::Numbers
                    | LoadSource::Hex => page(Topic::Words),
                    LoadSource::Backup => page(Topic::EncryptedBackups),
                    LoadSource::SeedXor => page(Topic::SeedXor),
                    // The shares of a SLIP-39 backup: the page about the
                    // forms a backup is written in, at its own section.
                    LoadSource::Slip39 => section(Topic::OtherBackups, other_backups::SLIP39),
                    // A codex32 string: the same page, at its own
                    // section (§16.109).
                    LoadSource::Codex32 => section(Topic::OtherBackups, other_backups::CODEX32),
                },
                LoadStep::PassphraseOffer | LoadStep::Passphrase | LoadStep::PassphraseConfirm => {
                    page(Topic::Passphrases)
                }
                // The count, the share's words and the result that says
                // what the set now has are all about SLIP-39.
                LoadStep::Count | LoadStep::Words | LoadStep::Checksum if w.is_slip39() => {
                    section(Topic::OtherBackups, other_backups::SLIP39)
                }
                // The string being typed and the result that says what
                // the set now has are both about codex32.
                LoadStep::Words | LoadStep::Checksum if w.is_codex32() => {
                    section(Topic::OtherBackups, other_backups::CODEX32)
                }
                _ => None,
            },
            Wizard::Build(w) => match w.step() {
                BuildStep::Kind => page(Topic::WalletKinds),
                BuildStep::Keys
                | BuildStep::Later
                | BuildStep::LaterThreshold
                | BuildStep::Delay
                | BuildStep::Days
                | BuildStep::Another
                | BuildStep::Script
                | BuildStep::Threshold
                | BuildStep::Review => section(Topic::WalletKinds, kind_section(w.kind())),
                BuildStep::Count
                | BuildStep::Quorum
                | BuildStep::Words
                | BuildStep::QuizStart
                | BuildStep::Quiz
                | BuildStep::QuizSkip
                | BuildStep::Record => page(Topic::Frost),
                _ => None,
            },
            Wizard::Backup(b) if b.step() == BackupStep::Shares => {
                section(Topic::OtherBackups, other_backups::SLIP39)
            }
            Wizard::Backup(b) if b.step() == BackupStep::Codex32 => {
                section(Topic::OtherBackups, other_backups::CODEX32)
            }
            Wizard::Backup(b) => match b.step() {
                BackupStep::Passphrase
                | BackupStep::PassphraseRepeat
                | BackupStep::Encrypted
                | BackupStep::EncryptedQr => page(Topic::EncryptedBackups),
                BackupStep::XorCount | BackupStep::XorSource | BackupStep::XorResult => {
                    page(Topic::SeedXor)
                }
                _ => None,
            },
        }
    }

    /// The page a screen is about, with no wizard over it.
    fn screen_target(&self) -> Option<LearnTarget> {
        match self.screen {
            // A key is 24 words and nothing more, and that is the page
            // the two lists of ways to one are about.
            // The applications BIP-85 derives are explained where
            // BIP-85 is, which is the page about a key's words
            // (§16.114).
            Screen::Keys | Screen::Add | Screen::OpenChild(_) | Screen::Bip85(_) => {
                page(Topic::Words)
            }
            Screen::OpenPassphrase(_) => page(Topic::Passphrases),
            Screen::KeyDetail(_) | Screen::BackupMenu(_) => page(Topic::Backups),
            Screen::Keep(_) => page(Topic::SecureElement),
            Screen::Wallets | Screen::AddWallet | Screen::WalletKeys(_) => page(Topic::WalletKinds),
            Screen::Wallet(w) => section(Topic::WalletKinds, self.wallet_section(w)),
            // §16.113: every screen a silent payments wallet has of its
            // own is about silent payments.
            Screen::SilentAddress(..)
            | Screen::SilentLabels(_)
            | Screen::SilentCheck(_)
            | Screen::SilentDns(_) => page(Topic::SilentPayments),
            // Export is about xpubs and what giving one away shows,
            // except when the format is a coordinator's own record.
            // §16.114: Bitcoin Core's import file is a coordinator's
            // file, as BIP 129's record is.
            Screen::Export(_)
                if matches!(
                    self.export_format(),
                    crate::ExportFormat::Bsms | crate::ExportFormat::CoreImport
                ) =>
            {
                page(Topic::Coordinators)
            }
            Screen::Export(_) => page(Topic::Xpubs),
            // §16.110: a key's account is an xpub and what giving one
            // away shows; BIP 129's key record and the two values it is
            // written from are a coordinator's file.
            Screen::KeyExport(_, _, step) => {
                if step != crate::KeyExportStep::Export
                    || self.export_format() == crate::ExportFormat::BsmsSigner
                {
                    page(Topic::Coordinators)
                } else {
                    page(Topic::Xpubs)
                }
            }
            // The keys a transaction names are about transactions, as
            // the review they were opened from is.
            Screen::SignKeys(_) => page(Topic::Transactions),
            Screen::Addresses(_) | Screen::Verify => page(Topic::Verifying),
            Screen::Inspect => page(Topic::Coordinators),
            Screen::SignMessage | Screen::CheckedMessage => page(Topic::Message),
            Screen::Scan => page(Topic::AirGap),
            Screen::Tools => page(Topic::Tools),
            // A note, a recovery sheet, the form an export takes and
            // the file it makes are all about the encrypted container
            // (`docs/PLANNING.md` §16.112).
            Screen::Notes
            | Screen::NoteText
            | Screen::Note
            | Screen::Sheet(_)
            | Screen::OpenedSheet
            | Screen::ExportForm
            | Screen::SealPass
            | Screen::Sealed
            | Screen::SealedQr => page(Topic::EncryptedBackups),
            // Two transactions compared is a check on what a signer
            // did, which is the page about nonces (§16.111).
            Screen::CompareTx => page(Topic::Nonces),
            Screen::Settings | Screen::About | Screen::Tiers => page(Topic::Devices),
            Screen::Sign => self.sign_target(),
            // A read transaction's own Signatures page is about the
            // same thing the signed one's is.
            Screen::Decode => match self.decode.stage() {
                Stage::Wizard(SignStep::Signatures) => page(Topic::Nonces),
                _ => None,
            },
            _ => None,
        }
    }

    /// The Sign flow: the review and the hold are about transactions,
    /// and the two results that are not a plain signature are about what
    /// made them what they are.
    fn sign_target(&self) -> Option<LearnTarget> {
        let Stage::Wizard(step) = self.sign.stage() else {
            return None;
        };
        match step {
            SignStep::Summary
            | SignStep::Outputs
            | SignStep::Inputs
            | SignStep::Warnings
            | SignStep::Confirm => page(Topic::Transactions),
            // What a signature is checked for, and which nonce rule
            // made it, is the page about nonces (§16.111).
            SignStep::Signatures => page(Topic::Nonces),
            SignStep::Result => {
                let outcome = self.sign.outcome()?;
                if outcome.threshold.is_some() {
                    page(Topic::Frost)
                } else if outcome.nonces_shared > 0 && !outcome.complete {
                    page(Topic::Nonces)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Which section of "Kinds of wallets" a wallet on screen is, read
    /// from the policy the same way its subtitle is.
    fn wallet_section(&self, wallet: WalletRef) -> usize {
        let policy = match wallet {
            WalletRef::Policy(i) => self.wallets.get(i),
            _ => None,
        };
        let Some(policy) = policy else {
            return kinds::SINGLE;
        };
        if policy.silent().is_some() {
            return kinds::SILENT;
        }
        if policy.tapscript_quorum().is_some() {
            return kinds::TAPROOT_MULTISIG;
        }
        match (policy.template(), policy.quorum()) {
            (_, Some(_)) => kinds::MULTISIG,
            (Template::MuSig, _) => kinds::MUSIG,
            (Template::Threshold { .. }, _) => kinds::FROST,
            (Template::Miniscript { .. } | Template::Tree, _) => kinds::RECOVERY,
            _ => kinds::SINGLE,
        }
    }
}
