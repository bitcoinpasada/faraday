//! The ? button: the Learn pages, read from `faraday_learn::pages::EN`
//! (edited as `docs/learn/*.md`), so Faraday shows the same text
//! and never a copy that
//! drifts. Each screen names the pages that explain it; a wallet's screens
//! name the pages for its kind.

use faraday_learn::Block;
use faraday_learn::Page as LearnPage;
use faraday_learn::pages::EN;

pub use faraday_learn::{
    GUIDES, MAKERS, Maker, backup_plan_check_page, demo_page, gpg_page, guide_address, guide_id,
    keys_kept_page, maker, page, secure_boot_keys_page, shares_page, time_page, vaults_page,
};

use crate::create::NewKind;
use crate::wallet::Kind;
use crate::{Faraday, Screen};

/// What the guide is opened on within a place (`docs/GUIDE-IN-APP.md` §1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideAt {
    /// The place as a whole.
    Whole,
    /// The open card of a flow, by the number the place's step module
    /// gives it (`cstep`, `kstep`, `rstep`, `qstep`, `mstep`, `step`).
    Step(u8),
    /// The shared backup flow's kind and step.
    Flow(crate::backup_flow::Kind, crate::backup_flow::Step),
    /// The shared backup flow of a kind as a whole, before a step.
    Kind(crate::backup_flow::Kind),
    /// The card showing what the flow is for, before its steps (the key a
    /// GPG restore is made against).
    Head,
    /// The sheet a place opens before its steps.
    Sheet,
    /// A code brought in to open, in the place of the card that makes one.
    Opening,
    /// An item of the backup checklist, by the number `bstep` gives it.
    Item(u8),
    /// A step of the flow a place holds in its column (the spend that
    /// Spend from a backup carries), by that flow's step module.
    Inner(u8),
    /// The tab, section, category or form the place has open, by its place
    /// in the place's own list: Settings' sections, a vault's categories,
    /// Add a key's forms (`entry::Form::ALL`).
    Part(u8),
}

/// Where the guide opens: the section key (`docs/GUIDE-IN-APP.md` §1.1)
/// of what `screen` shows at `at`. A lookup, done when the guide is
/// opened or the step changes, never while drawing. A step with no part
/// of its own gives the place's key.
pub fn guide_key(screen: Screen, at: GuideAt) -> &'static str {
    use crate::{mstep, qstep};
    let step = match at {
        GuideAt::Step(n) => Some(n),
        _ => None,
    };
    match screen {
        Screen::Home => "home",
        Screen::WalletsStart => "wallets-start",
        Screen::Wallets => "wallets",
        Screen::Spend => spend_key(step),
        Screen::Files => "files",
        Screen::Entry => entry_key(at),
        Screen::Backup => match (at, step) {
            (GuideAt::Item(n), _) => backup_item_key(n),
            (_, Some(qstep::SOFTWARE)) => "backup.software",
            (_, Some(qstep::PLAN)) => "backup.plan",
            (_, Some(qstep::BOARD)) => "backup.board",
            _ => "backup",
        },
        Screen::Message => match step {
            Some(mstep::ADDRESS) => "message.address",
            Some(mstep::TEXT) => "message.text",
            Some(mstep::FORMAT) => "message.format",
            Some(mstep::SIGN) => "message.sign",
            _ => "message",
        },
        Screen::CheckMessage => "check-message",
        Screen::Create => create_key(step),
        Screen::Restore => restore_key(step),
        Screen::Settings => settings_key(at),
        Screen::SetTime => "set-time",
        Screen::Randomness => "randomness",
        Screen::Vaults => "vaults",
        Screen::Backups => "backups",
        Screen::CreateVault => "create-vault",
        Screen::KeyGen => keygen_key(step),
        Screen::Bip85 => bip85_key(step),
        Screen::Convert => convert_key(step),
        Screen::KefTool => kef_tool_key(step),
        Screen::QrTool => qr_tool_key(at, step),
        Screen::OtsCheck => "ots-check",
        Screen::CoreWallet => match step {
            Some(crate::core_wallet::tool::cstep::FILE) => "core-wallet.file",
            Some(crate::core_wallet::tool::cstep::RESULT) => "core-wallet.result",
            _ => "core-wallet",
        },
        Screen::Silent => "silent",
        Screen::Explore => "explore",
        Screen::Lightning => "lightning",
        Screen::Tools => "tools",
        Screen::Unlock => "unlock",
        Screen::VaultContents => vault_contents_key(at),
        Screen::Codes => "codes",
        Screen::SpendBackup => spend_backup_key(at),
        Screen::Vanity => vanity_key(step),
        Screen::Decode => "decode",
        Screen::Invoice => "invoice",
        Screen::NonceCheck => "nonce-check",
        Screen::SignerCheck => "signer-check",
        Screen::Compare => "compare",
        Screen::Catalog => "catalog",
        Screen::Transfer => "transfer",
        Screen::BackupFlow => flow_key(at),
    }
}

/// The key of an Add a key form.
fn entry_key(at: GuideAt) -> &'static str {
    match at {
        GuideAt::Part(0) => "entry.words",
        GuideAt::Part(1) => "entry.shares",
        GuideAt::Part(2) => "entry.codex32",
        GuideAt::Part(3) => "entry.xor",
        _ => "entry",
    }
}

/// The key of a Settings section.
fn settings_key(at: GuideAt) -> &'static str {
    match at {
        GuideAt::Part(0) => "settings.mode",
        GuideAt::Part(1) => "settings.session",
        GuideAt::Part(2) => "settings.time",
        GuideAt::Part(3) => "settings.randomness",
        GuideAt::Part(4) => "settings.appearance",
        GuideAt::Part(5) => "settings.display",
        GuideAt::Part(6) => "settings.signed-amounts",
        GuideAt::Part(7) => "settings.nonce-history",
        GuideAt::Part(8) => "settings.about",
        _ => "settings",
    }
}

/// The key of a vault's category.
fn vault_contents_key(at: GuideAt) -> &'static str {
    match at {
        GuideAt::Part(0) => "vault-contents.bitcoin-keys",
        GuideAt::Part(1) => "vault-contents.wallets",
        GuideAt::Part(2) => "vault-contents.entries",
        GuideAt::Part(3) => "vault-contents.notes",
        GuideAt::Part(4) => "vault-contents.gpg-keys",
        GuideAt::Part(5) => "vault-contents.secure-boot",
        _ => "vault-contents",
    }
}

/// The key of a BIP-85 step.
fn bip85_key(step: Option<u8>) -> &'static str {
    match step {
        Some(crate::bip85::pstep::KEY) => "bip85.key",
        Some(crate::bip85::pstep::APP) => "bip85.app",
        Some(crate::bip85::pstep::INDEX) => "bip85.index",
        Some(crate::bip85::pstep::RESULT) => "bip85.result",
        _ => "bip85",
    }
}

/// The key of a QR vault step.
fn qr_tool_key(at: GuideAt, step: Option<u8>) -> &'static str {
    match (at, step) {
        (GuideAt::Opening, _) => "qr-tool.open",
        (_, Some(crate::qr_tool::qstep::CONTENTS)) => "qr-tool.contents",
        (_, Some(crate::qr_tool::qstep::PASSPHRASE)) => "qr-tool.passphrase",
        (_, Some(crate::qr_tool::qstep::CODE)) => "qr-tool.code",
        _ => "qr-tool",
    }
}

/// The key of a convert step.
fn convert_key(step: Option<u8>) -> &'static str {
    match step {
        Some(crate::convert::cstep::FROM) => "convert.from",
        Some(crate::convert::cstep::TO) => "convert.to",
        Some(crate::convert::cstep::SETTINGS) => "convert.settings",
        Some(crate::convert::cstep::RESULT) => "convert.result",
        _ => "convert",
    }
}

/// The key of a Krux code step.
fn kef_tool_key(step: Option<u8>) -> &'static str {
    match step {
        Some(crate::kef_tool::kstep::CONTENTS) => "kef-tool.contents",
        Some(crate::kef_tool::kstep::MAKE) => "kef-tool.make",
        Some(crate::kef_tool::kstep::CODE) => "kef-tool.code",
        _ => "kef-tool",
    }
}

/// The key of a backup checklist item, by its `bstep` number.
fn backup_item_key(n: u8) -> &'static str {
    use crate::plan::Item;
    match crate::bstep::item(n) {
        Some(Item::Templates) => "backup.blank",
        Some(Item::Copy(_)) => "backup.copy",
        Some(Item::Codex32(_)) => "backup.codex32",
        Some(Item::Passphrase(_)) => "backup.passphrase",
        Some(Item::KefCode(_)) => "backup.kef",
        Some(Item::Vault(_)) => "backup.vault",
        Some(Item::SeedFiles) => "backup.files",
        Some(Item::Sheets) => "backup.sheets",
        Some(Item::PublicFiles) => "backup.public",
        Some(Item::ShowDescriptor) => "backup.show",
        Some(Item::Envelopes) => "backup.envelope",
        Some(Item::Sticks) => "backup.sticks",
        None => "backup",
    }
}

/// The key of a card of Spend from a backup: one of the tab's pages, or
/// a step of the spend in its column. The start from the stick and the
/// open wallet are folded into what you have; the spend's wallet is
/// chosen there too, and its first addresses are the check.
fn spend_backup_key(at: GuideAt) -> &'static str {
    use crate::spend_backup::page;
    use crate::wallet::step;
    match at {
        GuideAt::Step(page::MAP) => "spend-from-backup.map",
        GuideAt::Step(page::SAFE | page::HOLDING | page::OPEN) => "spend-from-backup.holding",
        GuideAt::Step(page::CHECK) => "spend-from-backup.check",
        GuideAt::Step(page::WRITE) => "spend-from-backup.write",
        GuideAt::Step(page::BRING) => "spend-from-backup.bring",
        GuideAt::Step(page::AWAY) => "spend-from-backup.away",
        GuideAt::Inner(step::WALLET) => "spend-from-backup.holding",
        GuideAt::Inner(step::CHECK) => "spend-from-backup.check",
        GuideAt::Inner(step::TRANSACTION) => "spend-from-backup.transaction",
        GuideAt::Inner(step::TXID) => "spend-from-backup.txid",
        GuideAt::Inner(step::PATH) => "spend-from-backup.path",
        GuideAt::Inner(step::NONCES) => "spend-from-backup.nonces",
        GuideAt::Inner(step::SIGNERS) => "spend-from-backup.signers",
        GuideAt::Inner(step::SIGN) => "spend-from-backup.sign",
        GuideAt::Inner(step::COLLECT) => "spend-from-backup.collect",
        GuideAt::Inner(step::FINISH) => "spend-from-backup.finish",
        _ => "spend-from-backup",
    }
}

/// The key of a spend step.
fn spend_key(step: Option<u8>) -> &'static str {
    match step {
        Some(crate::wallet::step::WALLET) => "spend.wallet",
        Some(crate::wallet::step::CHECK) => "spend.check",
        Some(crate::wallet::step::TRANSACTION) => "spend.transaction",
        Some(crate::wallet::step::TXID) => "spend.txid",
        Some(crate::wallet::step::SIGNERS) => "spend.signers",
        Some(crate::wallet::step::SIGN) => "spend.sign",
        Some(crate::wallet::step::COLLECT) => "spend.collect",
        Some(crate::wallet::step::FINISH) => "spend.finish",
        Some(crate::wallet::step::PATH) => "spend.path",
        Some(crate::wallet::step::NONCES) => "spend.nonces",
        _ => "spend",
    }
}

/// The key of a create step.
fn create_key(step: Option<u8>) -> &'static str {
    use crate::cstep;
    match step {
        Some(cstep::KIND) => "create.kind",
        Some(cstep::QUORUM) => "create.quorum",
        Some(cstep::KEYS) => "create.keys",
        Some(cstep::CHECK) => "create.check",
        Some(cstep::BACKUP) => "create.backup",
        Some(cstep::WHERE) => "create.where",
        Some(cstep::STICKS) => "create.sticks",
        Some(cstep::VAULTS) => "create.vaults",
        Some(cstep::HOW_MANY) => "create.how-many",
        Some(cstep::BACKUP_FORM) => "create.backup-form",
        _ => "create",
    }
}

/// The key of a restore step.
fn restore_key(step: Option<u8>) -> &'static str {
    use crate::rstep;
    match step {
        Some(rstep::KIND) => "restore.kind",
        Some(rstep::QUORUM) => "restore.quorum",
        Some(rstep::DESCRIPTION) => "restore.description",
        Some(rstep::SEEDS) => "restore.seeds",
        Some(rstep::CHECK) => "restore.check",
        Some(rstep::DONE) => "restore.done",
        Some(rstep::WHERE) => "restore.where",
        Some(rstep::HOW_MANY) => "restore.how-many",
        _ => "restore",
    }
}

/// The key of a keygen step.
fn keygen_key(step: Option<u8>) -> &'static str {
    match step {
        Some(crate::keygen::kstep::START) => "keygen.start",
        Some(crate::keygen::kstep::LENGTH) => "keygen.length",
        Some(crate::keygen::kstep::SOURCE) => "keygen.source",
        Some(crate::keygen::kstep::ENTER) => "keygen.enter",
        Some(crate::keygen::kstep::KEY) => "keygen.key",
        Some(crate::keygen::kstep::CHECK) => "keygen.check",
        Some(crate::keygen::kstep::WORDS) => "keygen.words",
        Some(crate::keygen::kstep::QUIZ) => "keygen.quiz",
        Some(crate::keygen::kstep::TYPED) => "keygen.typed",
        Some(crate::keygen::kstep::CODEX32) => "keygen.codex32",
        _ => "keygen",
    }
}

/// The key of a vanity step.
fn vanity_key(step: Option<u8>) -> &'static str {
    match step {
        Some(crate::vanity::vstep::KEY) => "vanity.key",
        Some(crate::vanity::vstep::DIAL) => "vanity.dial",
        Some(crate::vanity::vstep::SCRIPT) => "vanity.script",
        Some(crate::vanity::vstep::PREFIX) => "vanity.prefix",
        Some(crate::vanity::vstep::RUN) => "vanity.run",
        _ => "vanity",
    }
}

/// The key of a step of the shared backup flow.
fn flow_key(at: GuideAt) -> &'static str {
    use crate::backup_flow::{Kind as FlowKind, Step as FlowStep};
    match at {
        GuideAt::Flow(FlowKind::Gpg, s) => match s {
            FlowStep::Where => "gpg-restore.where",
            FlowStep::Sticks => "gpg-restore.sticks",
            FlowStep::Vaults => "gpg-restore.vaults",
            FlowStep::Make => "gpg-restore.make",
            FlowStep::Lock => "gpg-restore.lock",
            FlowStep::Write => "gpg-restore.write",
            FlowStep::Done => "gpg-restore.done",
        },
        GuideAt::Flow(FlowKind::SecureBoot, s) => match s {
            FlowStep::Where => "secure-boot-keys.where",
            FlowStep::Sticks => "secure-boot-keys.sticks",
            FlowStep::Vaults => "secure-boot-keys.vaults",
            FlowStep::Make => "secure-boot-keys.make",
            FlowStep::Lock => "secure-boot-keys.lock",
            FlowStep::Write => "secure-boot-keys.write",
            FlowStep::Done => "secure-boot-keys.done",
        },
        GuideAt::Flow(FlowKind::Wallet, s) => match s {
            FlowStep::Where => "backup-flow.where",
            FlowStep::Sticks => "backup-flow.sticks",
            FlowStep::Vaults => "backup-flow.vaults",
            FlowStep::Make => "backup-flow.make",
            FlowStep::Lock => "backup-flow.lock",
            FlowStep::Write => "backup-flow.write",
            FlowStep::Done => "backup-flow.done",
        },
        GuideAt::Kind(FlowKind::Gpg) => "gpg-restore",
        GuideAt::Kind(FlowKind::SecureBoot) => "secure-boot-keys",
        GuideAt::Head => "gpg-restore.head",
        GuideAt::Sheet => "gpg-restore.sheet",
        _ => "backup-flow",
    }
}

/// The section Back up's Extra copies card opens, in Guided only
/// (`docs/GUIDE-IN-APP.md` §4): a card shown under the plan, not a step.
pub const EXTRA_COPIES_KEY: &str = "backup.extra-copies";

/// Where a key resolves in the written pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideAim {
    /// The key's own section.
    Section(&'static str),
    /// The place's lead section: the part has none.
    Lead(&'static str),
    /// The guide's contents: the place has no page.
    Contents,
}

/// Where `key` opens: its section if written, else its place's lead,
/// else the contents (`docs/GUIDE-IN-APP.md` §1.2). The guide never
/// opens empty.
pub fn guide_aim(key: &'static str) -> GuideAim {
    let place = key.split('.').next().unwrap_or(key);
    if faraday_learn::guide_section(key).is_some() {
        GuideAim::Section(key)
    } else if faraday_learn::guide_page(place).is_some() {
        GuideAim::Lead(place)
    } else {
        GuideAim::Contents
    }
}

/// The label of `page`'s tab in the Learn sheet and pane: its title, or
/// a shorter name where the title is too long for a row of tabs to show
/// whole beside the others (Material 3 Tabs). A lookup.
pub fn tab_label(page: &'static LearnPage) -> &'static str {
    if std::ptr::eq(page, keys_kept_page()) {
        crate::strings::EN.learn_tab_keys_kept
    } else {
        page.title
    }
}

/// The tab number the How Faraday does it tab is pressed as
/// ([`crate::Action::LearnPage`]): no page has it.
pub const HOW_TAB: u8 = u8::MAX;

/// A Learn page a Guided card links to, named by what it explains. Merged
/// pages are named by the page they are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    /// Start here.
    StartHere,
    /// Words.
    Words,
    /// Randomness.
    Randomness,
    /// Backups.
    Backups,
    /// Backups in other forms (SLIP-39, codex32, Seed XOR).
    OtherBackups,
    /// Passphrases.
    Passphrases,
    /// Verifying.
    Verifying,
    /// The air gap.
    AirGap,
    /// Multisig.
    Multisig,
    /// Spend paths and timelocks.
    SpendPaths,
    /// Xpubs and privacy.
    Xpubs,
    /// Nonces.
    Nonces,
    /// Signing a message.
    Message,
    /// Tools.
    Tools,
    /// Kinds of wallets.
    WalletKinds,
    /// Coordinator files.
    Coordinators,
    /// Silent payments.
    SilentPayments,
    /// Lightning invoices.
    LightningInvoices,
    /// Timestamps.
    Timestamps,
    /// Bitcoin Core wallets.
    CoreWallet,
    /// Shares of a wallet description.
    Shares,
    /// A backup plan's check.
    BackupPlanCheck,
    /// Vaults.
    Vaults,
    /// Wiping a stick, from the wipe sheet.
    WipingAStick,
    /// Extra copies, from the backup's last card.
    ExtraCopies,
}

impl Topic {
    /// Every topic, in declaration order.
    const ALL: &[Topic] = &[
        Topic::StartHere,
        Topic::Words,
        Topic::Randomness,
        Topic::Backups,
        Topic::OtherBackups,
        Topic::Passphrases,
        Topic::Verifying,
        Topic::AirGap,
        Topic::Multisig,
        Topic::SpendPaths,
        Topic::Xpubs,
        Topic::Nonces,
        Topic::Message,
        Topic::Tools,
        Topic::WalletKinds,
        Topic::Coordinators,
        Topic::SilentPayments,
        Topic::LightningInvoices,
        Topic::Timestamps,
        Topic::CoreWallet,
        Topic::Shares,
        Topic::BackupPlanCheck,
        Topic::Vaults,
        Topic::WipingAStick,
    ];

    /// The label of the link to this page, made once and kept: drawing
    /// reads it and allocates nothing.
    pub fn label(self) -> &'static str {
        static LABELS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
        let labels = LABELS.get_or_init(|| {
            let make = |t: Topic| {
                crate::strings::fill(faraday_strings::EN.guide_learn_link, &[t.page().title])
            };
            Topic::ALL
                .iter()
                .copied()
                .chain([Topic::ExtraCopies])
                .map(make)
                .collect()
        });
        &labels[Topic::ALL
            .iter()
            .position(|&t| t == self)
            .unwrap_or(Topic::ALL.len())]
    }

    /// The page this topic is.
    pub fn page(self) -> &'static LearnPage {
        match self {
            Topic::StartHere => &EN.start_here,
            Topic::Words => &EN.words,
            Topic::Randomness => &EN.randomness,
            Topic::Backups => &EN.backups,
            Topic::OtherBackups => &EN.other_backups,
            Topic::Passphrases => &EN.passphrases,
            Topic::Verifying => &EN.verifying,
            Topic::AirGap => &EN.air_gap,
            Topic::Multisig => &EN.multisig,
            Topic::SpendPaths => &EN.spend_paths,
            Topic::Xpubs => &EN.xpubs,
            Topic::Nonces => &EN.nonces,
            Topic::Message => &EN.message,
            Topic::Tools => &EN.tools,
            Topic::WalletKinds => &EN.wallet_kinds,
            Topic::Coordinators => &EN.coordinators,
            Topic::SilentPayments => &EN.silent_payments,
            Topic::LightningInvoices => &EN.lightning_invoices,
            Topic::Timestamps => &EN.timestamps,
            Topic::CoreWallet => &EN.core_wallet,
            Topic::Shares => shares_page(),
            Topic::BackupPlanCheck => backup_plan_check_page(),
            Topic::Vaults => vaults_page(),
            Topic::WipingAStick => faraday_learn::wiping_a_stick_page(),
            Topic::ExtraCopies => faraday_learn::extra_copies_page(),
        }
    }
}

/// The Learn sheet's state: which pages, which one is open, and how far
/// it is scrolled.
#[derive(Default)]
pub struct LearnState {
    /// The pages the screen named.
    pub pages: Vec<&'static LearnPage>,
    /// The one on show.
    pub page: usize,
    /// Design units scrolled.
    pub scroll: f32,
    /// The furthest it can scroll, as last drawn.
    pub max: f32,
    /// The How Faraday does it tab is on show, in place of the page
    /// (`docs/CRYPTO-LEARN.md` §8.2). Kept only while the screen offers
    /// blocks ([`Faraday::learn_place`]).
    pub how: bool,
}

impl LearnState {
    /// The tab on show, by its place among the tabs the sheet draws when
    /// the How tab is there: 0 for How, else the page's place from 1.
    /// A lookup.
    pub fn tab(&self) -> usize {
        if self.how { 0 } else { self.page + 1 }
    }
}

fn for_kind(kind: Kind) -> Vec<&'static LearnPage> {
    match kind {
        Kind::Single(_) => vec![&EN.wallet_kinds, &EN.words],
        Kind::Multi(_) => vec![&EN.multisig, &EN.xpubs, &EN.coordinators],
        Kind::TapMulti => vec![&EN.multisig, &EN.spend_paths],
        Kind::Miniscript | Kind::Tree => {
            vec![&EN.spend_paths, &EN.inheritance, &EN.wallet_kinds]
        }
        Kind::MuSig => vec![&EN.nonces, &EN.multisig],
        Kind::Silent => vec![&EN.silent_payments, &EN.wallet_kinds],
        Kind::Other => vec![&EN.wallet_kinds],
    }
}

fn for_new(kind: NewKind) -> Vec<&'static LearnPage> {
    match kind {
        NewKind::NativeSegwit | NewKind::Taproot | NewKind::NestedSegwit | NewKind::Legacy => {
            vec![&EN.wallet_kinds, &EN.words, &EN.backups]
        }
        NewKind::Multi | NewKind::MultiNested | NewKind::MultiLegacy => {
            vec![&EN.multisig, &EN.xpubs, &EN.coordinators]
        }
        NewKind::TapMulti => vec![&EN.multisig, &EN.spend_paths],
        NewKind::MuSig => vec![&EN.nonces, &EN.wallet_kinds],
    }
}

impl Faraday {
    /// The Learn pages that explain what is on screen. Where the demo is
    /// set up (F20), the screens where keys come in and the mode is
    /// chosen name its page first, and so, while the demo is on (where
    /// its actions are dimmed), does every screen with an action the demo
    /// dims (`docs/DEMO.md` §4, §7). The pages of the actions the screen
    /// offers follow its own.
    pub fn learn_pages(&self) -> Vec<&'static LearnPage> {
        let mut pages = self.screen_pages();
        // The page of each action the screen offers, after the curated ones
        // (`docs/CRYPTO-LEARN.md` §9.1).
        if !self.held {
            let name = self.how_screen_name();
            for block in faraday_learn::how::screen_blocks(name.as_str()) {
                let page = faraday_learn::how::learn_page(block.learn);
                if let Some(page) = page.filter(|p| !pages.iter().any(|q| std::ptr::eq(*q, *p))) {
                    pages.push(page);
                }
            }
        }
        let keys_in = self.session.keys.demo()
            && matches!(
                self.screen,
                Screen::Home | Screen::Entry | Screen::KeyGen | Screen::Settings
            );
        let dims = self.session.keys.demo()
            && (matches!(
                self.screen,
                Screen::BackupFlow
                    | Screen::VaultContents
                    | Screen::Backup
                    | Screen::Bip85
                    | Screen::Create
                    | Screen::Vanity
                    | Screen::Lightning
                    | Screen::Silent
                    | Screen::CreateVault
            ) || self.sheet == Some(crate::Sheet::Chart));
        if keys_in || dims {
            pages.insert(0, demo_page());
        }
        pages
    }

    fn screen_pages(&self) -> Vec<&'static LearnPage> {
        let wallet = || {
            self.session
                .wallets
                .get(self.wallets.wallet())
                .map(|w| Kind::of(w.policy()))
        };
        match self.screen {
            Screen::Home => vec![&EN.start_here, &EN.air_gap],
            Screen::SpendBackup => vec![&EN.inheritance, &EN.verifying, &EN.air_gap, &EN.scams],
            Screen::WalletsStart => vec![&EN.wallet_kinds, &EN.start_here],
            Screen::Wallets => wallet().map_or_else(|| vec![&EN.wallet_kinds], for_kind),
            Screen::Spend => {
                let mut p = vec![&EN.verifying];
                if wallet() == Some(Kind::MuSig) {
                    p.push(&EN.nonces);
                }
                p.push(&EN.scams);
                p
            }
            Screen::Create => self
                .create
                .state()
                .map_or_else(|| vec![&EN.wallet_kinds], |c| for_new(c.kind)),
            // Make codex32 shares and the codex32 form name the codex32
            // page first (`docs/CODEX32.md` §13.3).
            Screen::KeyGen if self.keygen.state().is_some_and(|k| k.codex32.is_some()) => {
                vec![&EN.other_backups, &EN.randomness]
            }
            Screen::KeyGen => vec![&EN.randomness, &EN.words],
            Screen::Randomness => vec![&EN.randomness],
            Screen::Entry if self.entry.state().form == crate::forms::Form::Codex32 => {
                vec![&EN.other_backups, &EN.words]
            }
            Screen::Entry => vec![&EN.words, &EN.passphrases],
            Screen::Silent => vec![&EN.silent_payments, &EN.xpubs],
            Screen::Explore => vec![&EN.xpubs, &EN.glossary, keys_kept_page()],
            Screen::Vanity => vec![&EN.passphrases, &EN.wallet_kinds],
            Screen::Lightning => vec![&EN.tools, &EN.glossary],
            Screen::Tools => vec![&EN.tools, &EN.glossary],
            Screen::OtsCheck => vec![&EN.timestamps, &EN.tools],
            Screen::CoreWallet => vec![&EN.core_wallet, &EN.tools],
            Screen::Bip85 | Screen::Convert | Screen::KefTool | Screen::QrTool => {
                vec![&EN.words, &EN.passphrases, &EN.tools, &EN.glossary]
            }
            Screen::Backup => {
                // A multisig's backup can keep shares of its description.
                let splits = self
                    .backup
                    .state()
                    .and_then(|b| self.session.wallets.get(b.wallet))
                    .is_some_and(crate::wallet_backup::splits);
                if splits {
                    vec![
                        &EN.backups,
                        backup_plan_check_page(),
                        shares_page(),
                        &EN.other_backups,
                    ]
                } else {
                    vec![&EN.backups, backup_plan_check_page(), &EN.other_backups]
                }
            }
            Screen::Backups => vec![&EN.backups, backup_plan_check_page(), vaults_page()],
            Screen::Restore => vec![&EN.backups, backup_plan_check_page(), &EN.inheritance],
            Screen::Message | Screen::CheckMessage => vec![&EN.message],
            Screen::Files => vec![&EN.air_gap, &EN.coordinators],
            Screen::VaultContents if self.vaults.category == 4 => {
                vec![gpg_page(), vaults_page(), &EN.passphrases]
            }
            // A GPG key restored from its paperkey: the page that says how
            // its lines are typed.
            Screen::BackupFlow
                if self
                    .flows
                    .on()
                    .and_then(|i| self.flows.get(i))
                    .is_some_and(|f| f.restore.is_some()) =>
            {
                vec![gpg_page(), vaults_page(), &EN.passphrases]
            }
            // The Secure Boot kind (`crate::vault::CATEGORIES`, place 5).
            Screen::VaultContents if self.vaults.category == 5 => {
                vec![secure_boot_keys_page(), vaults_page(), &EN.passphrases]
            }
            Screen::Vaults
            | Screen::CreateVault
            | Screen::Unlock
            | Screen::BackupFlow
            | Screen::VaultContents => {
                vec![vaults_page(), &EN.passphrases]
            }
            Screen::Settings => vec![keys_kept_page(), &EN.glossary],
            Screen::SetTime | Screen::Codes => vec![time_page()],
            Screen::Decode => vec![&EN.verifying, &EN.tools],
            Screen::Invoice => vec![&EN.lightning_invoices, &EN.tools],
            Screen::NonceCheck => vec![&EN.nonces, &EN.verifying],
            Screen::SignerCheck => vec![&EN.verifying, &EN.tools],
            Screen::Compare => vec![&EN.verifying, &EN.tools],
            Screen::Catalog => vec![&EN.tools, &EN.glossary],
            Screen::Transfer => vec![&EN.air_gap, &EN.coordinators],
        }
    }

    /// The ? and Learn in the navigation open Learn on the screen's
    /// first page, as the sheet or, on an extra-large window, the
    /// supporting pane beside the page; pressed again while the pane is
    /// shown, and the pane's close button, put the pane away.
    pub(crate) fn learn_press(&mut self, action: crate::Action) {
        if action == crate::Action::How {
            self.how_press();
        } else if action == crate::Action::LearnClose || self.learn_pane_shown() {
            self.learn_close();
        } else {
            self.learn_open();
            self.learn_place();
        }
    }

    /// **How Faraday does it** on the bar: opens Learn on the How tab;
    /// while the supporting pane shows that tab it puts the pane away,
    /// and while the pane shows another tab it moves the pane to the How
    /// tab (`docs/CRYPTO-LEARN.md` §8.2).
    pub(crate) fn how_press(&mut self) {
        if !self.how_offered() {
            return;
        }
        if self.learn_pane_shown() {
            if self.learn.state().how {
                self.learn_close();
            } else {
                self.learn.show_how();
            }
        } else {
            self.learn_open();
            self.learn.show_how();
            self.learn_place();
        }
    }

    /// The screens' name for the blocks to find it by: its variant's
    /// name, in a buffer on the stack. A lookup.
    fn how_screen_name(&self) -> ScreenName {
        use std::fmt::Write;
        let mut name = ScreenName::default();
        // not shown: the variant's name, which the blocks list their screens by
        let _ = write!(name, "{:?}", self.screen);
        name
    }

    /// The blocks of the actions the screen on show offers, in the order
    /// the manifest lists them (`docs/CRYPTO-LEARN.md` §8.2, §9.2); none
    /// in a held app, whose holder has its own. A lookup.
    pub fn how_blocks(&self) -> Vec<&'static Block> {
        if self.held {
            return Vec::new();
        }
        faraday_learn::how::for_screen(self.how_screen_name().as_str())
    }

    /// Whether the screen on show offers an action with a block: the bar
    /// then carries **How Faraday does it** and Learn a How tab. A
    /// lookup.
    pub fn how_offered(&self) -> bool {
        !self.held && faraday_learn::how::offered(self.how_screen_name().as_str())
    }

    /// Opens the Learn sheet on the screen's first page.
    fn learn_open(&mut self) {
        let pages = self.learn_pages();
        if pages.is_empty() && !self.how_offered() {
            return;
        }
        self.learn.open(pages, &mut self.sheet);
    }

    /// The pane's pin, which turns it on or off, or **Spending, step by
    /// step** over Learn's tabs.
    pub(crate) fn learn_button(&mut self, action: crate::Action) {
        if action == crate::Action::LearnPin {
            self.learn.toggle_pin(self.screen);
        } else {
            self.learn_spend();
        }
    }

    /// The Learn sheet's and pane's first row on the Spend tab,
    /// **Spending, step by step**: Learn closes, sheet or pane, and the
    /// Spend tab's steps are on show.
    fn learn_spend(&mut self) {
        self.sheet = None;
        self.learn_close();
        self.screen = Screen::SpendBackup;
    }

    /// Whether the window holds Learn as a supporting pane beside the
    /// page (`docs/LAYOUT.md` §6): an extra-large window (1600 dp and
    /// over) beside the sidebar, not held. Below it Learn is the sheet.
    /// A lookup.
    fn learn_pane_room(&self) -> bool {
        !self.is_compact()
            && !self.held
            && self.window().class() == faraday_draw::geom::WindowClass::ExtraLarge
    }

    /// Whether the screen on show leaves the supporting pane's place to
    /// Learn: one pane stands at a time, so the codex32 entry (its dice
    /// sheet's column) and Back up and a wallet's backup flow (the
    /// board's lists) keep the place for their own, and Learn is the
    /// sheet there (`docs/LAYOUT.md` §17, §19). A lookup.
    fn learn_pane_place_free(&self) -> bool {
        let codex32 = self.screen == Screen::KeyGen
            && self.keygen.state().is_some_and(|k| k.codex32.is_some());
        !codex32 && !matches!(self.screen, Screen::Backup | Screen::BackupFlow)
    }

    /// Whether Learn stands as the supporting pane this frame: open as
    /// the pane, pinned or on the screen it was opened beside, on a
    /// window and a screen that hold it, with no chart's side sheet in
    /// the pane's place. A lookup.
    pub(crate) fn learn_pane_shown(&self) -> bool {
        self.learn.pane_open()
            && self.learn.stays_beside(self.screen)
            && self.learn_pane_room()
            && self.learn_pane_place_free()
            && crate::glance::sheet::side_now(self).is_none()
    }

    /// Puts the supporting pane away: by its close button, Escape, the
    /// mouse's back button, or the ? pressed again. The wheel and the
    /// arrow keys move the page again.
    pub(crate) fn learn_close(&mut self) {
        self.learn.close_pane();
        self.scroll.went_to(crate::ui::Slot::Page);
    }

    /// Cancel's part in Learn: the open sheet closes; with no sheet open,
    /// Learn's supporting pane is put away, as Cancel closes the sheet.
    pub(crate) fn learn_cancel(&mut self) {
        if self.sheet.is_none() {
            self.learn_close();
        }
        self.sheet = None;
    }

    /// Escape while the supporting pane is shown and no sheet is over
    /// the page puts the pane away, as it closes the sheet. True when it
    /// took the key.
    pub(crate) fn learn_key(&mut self, key: faraday_shell_api::Key) -> bool {
        let take = key == faraday_shell_api::Key::Escape
            && self.sheet.is_none()
            && self.learn_pane_shown();
        if take {
            self.learn_close();
        }
        take
    }

    /// Keeps Learn where the window and the screen put it, and the pane's
    /// pages the screen's: called before every frame is drawn and after
    /// Learn opens. The sheet becomes the pane once the window holds one
    /// (extra-large), and the pane the sheet once it does not, unless
    /// another sheet is up; a pane on a screen that keeps the pane's
    /// place for its own goes, and so does an unpinned pane once the
    /// screen is another than the one it stood beside (owner,
    /// 2026-10-29). While the pane is open its pages are the
    /// Learn pages of the screen on show ([`Faraday::learn_pages`]),
    /// worked out again here and compared with the ones it last
    /// followed, so a change of screen or step changes them however it
    /// came; the tab on show stays where the new pages have it, else the
    /// first is shown. Costs a lookup of the screen's pages while Learn
    /// is open.
    pub(crate) fn learn_place(&mut self) {
        if self.learn.state().how && !self.how_offered() {
            self.learn.drop_how();
        }
        if self.learn.pane_open() {
            if self.sheet.is_some() {
                return;
            }
            if !self.learn_pane_room() {
                self.learn.move_to_sheet(&mut self.sheet);
                self.scroll.went_to(crate::ui::Slot::Page);
                return;
            }
            // Unpinned, it goes when the person moves to another screen.
            if self.learn.left_screen(self.screen) {
                self.learn_close();
                return;
            }
            let now = self.learn_pages();
            if (now.is_empty() && !self.how_offered()) || !self.learn_pane_place_free() {
                self.learn_close();
            } else {
                self.learn.follow(now);
            }
        } else if self.sheet == Some(crate::Sheet::Learn)
            && self.learn_pane_room()
            && self.learn_pane_place_free()
        {
            let now = self.learn_pages();
            self.learn.move_to_pane(&mut self.sheet, now, self.screen);
        }
    }

    /// Opens the Learn sheet on a Guided card's topic.
    pub(crate) fn learn_topic(&mut self, topic: Topic) {
        self.learn_on(topic.page());
    }

    /// Opens the Learn sheet on the page about shares; closing it leaves
    /// the screen as it was.
    pub(crate) fn learn_shares(&mut self) {
        self.learn_on(shares_page());
    }

    /// The Learn sheet on `page`: on its tab where the screen's pages
    /// have it, else on it alone.
    pub(crate) fn learn_on(&mut self, page: &'static LearnPage) {
        if !self.learn.pane_open() {
            self.learn_open();
        }
        self.learn.on(page, &mut self.sheet);
        self.learn_place();
    }

    /// Opens the Learn sheet on `pages` over the screen on show: for the
    /// app holding this one ([`Faraday::held`]), whose own screens name
    /// the pages. Closing the sheet leaves the screen as it was.
    pub fn learn_with(&mut self, pages: Vec<&'static LearnPage>) {
        if pages.is_empty() {
            return;
        }
        self.learn.open(pages, &mut self.sheet);
        self.out.redraw();
        self.out.command(faraday_shell_api::Command::Draw);
    }

    /// The Learn sheet's state: its pages, the one on show and its scroll.
    pub fn learn(&self) -> &LearnState {
        self.learn.state()
    }

    /// Whether Learn's pane is pinned open across screens.
    pub fn learn_pinned(&self) -> bool {
        self.learn.pinned()
    }

    /// The steps ticked on `page` this session, in order.
    /// Kept in memory only: leaving and
    /// reopening the page in the same session keeps them; a new session
    /// starts unticked.
    pub fn learn_ticked(&self, page: &'static LearnPage) -> &[u32] {
        self.learn.ticked(page)
    }
}

/// The Learn sheet's area: its state and the steps ticked in this
/// session.
#[derive(Default)]
pub(crate) struct LearnArea {
    state: LearnState,
    /// The steps ticked on each guide page the Learn sheet has shown
    /// this session, by page; a page with none ticked is not here.
    /// Session state only, in memory, never written to a file.
    ticks: Vec<(&'static LearnPage, Vec<u32>)>,
    /// Learn is open as the supporting pane beside the page, on an
    /// extra-large window (`docs/LAYOUT.md` §6), not as the sheet.
    pane: bool,
    /// The pane's pin is on: the pane stays open as the person moves to
    /// another screen. Session state only, in memory; off whenever the
    /// pane is not open.
    pinned: bool,
    /// The screen the pane last stood beside, kept in step with the
    /// screen by [`Faraday::learn_place`] before every frame. `None`
    /// while the pane is not open.
    beside: Option<crate::Screen>,
    /// The screen's Learn pages as the pane last followed them. Kept in
    /// step with the screen by [`Faraday::learn_place`] before every
    /// frame: once the screen's pages are no longer these, the pane's
    /// pages become the new ones. Empty while the pane is not open.
    followed: Vec<&'static LearnPage>,
}

impl LearnArea {
    /// The sheet's state.
    pub(crate) fn state(&self) -> &LearnState {
        &self.state
    }

    /// The sheet's scroll offset.
    pub(crate) fn scroll_mut(&mut self) -> &mut f32 {
        &mut self.state.scroll
    }

    /// Takes the extent and offset a frame drew the sheet with.
    pub(crate) fn set_scroll(&mut self, max: f32, scroll: f32) {
        self.state.max = max;
        self.state.scroll = scroll;
    }

    /// Opens the sheet on `pages`, at the first, and shows it.
    pub(crate) fn open(
        &mut self,
        pages: Vec<&'static LearnPage>,
        sheet: &mut Option<crate::Sheet>,
    ) {
        self.state = LearnState {
            pages,
            ..LearnState::default()
        };
        self.close_pane();
        *sheet = Some(crate::Sheet::Learn);
    }

    /// Whether Learn is open as the supporting pane.
    pub(crate) fn pane_open(&self) -> bool {
        self.pane
    }

    /// The open sheet becomes the pane, on the pages and tab it shows,
    /// beside `on`; `screen` is the screen's pages now, which the pane
    /// follows from here.
    pub(crate) fn move_to_pane(
        &mut self,
        sheet: &mut Option<crate::Sheet>,
        screen: Vec<&'static LearnPage>,
        on: crate::Screen,
    ) {
        *sheet = None;
        self.pane = true;
        self.beside = Some(on);
        self.followed = screen;
    }

    /// Whether the pane's pin is on.
    pub(crate) fn pinned(&self) -> bool {
        self.pinned
    }

    /// The pin pressed on the pane beside `on`: on, the pane stays open
    /// across screens; off, it goes at the next one.
    pub(crate) fn toggle_pin(&mut self, on: crate::Screen) {
        if self.pane {
            self.pinned = !self.pinned;
            self.beside = Some(on);
        }
    }

    /// Whether the pane still stands beside `screen`: pinned, or on the
    /// screen it last stood beside.
    pub(crate) fn stays_beside(&self, screen: crate::Screen) -> bool {
        self.pinned || self.beside.is_none_or(|s| s == screen)
    }

    /// Notes that the pane stands beside `screen` now. True when the
    /// pane is unpinned and that is another screen than the one it stood
    /// beside: it is then put away.
    pub(crate) fn left_screen(&mut self, screen: crate::Screen) -> bool {
        let left = !self.stays_beside(screen);
        self.beside = Some(screen);
        left
    }

    /// The pane becomes the sheet, on the pages and tab it shows.
    pub(crate) fn move_to_sheet(&mut self, sheet: &mut Option<crate::Sheet>) {
        self.close_pane();
        *sheet = Some(crate::Sheet::Learn);
    }

    /// The pane is put away, and its pin taken off.
    pub(crate) fn close_pane(&mut self) {
        self.pane = false;
        self.pinned = false;
        self.beside = None;
        self.followed.clear();
    }

    /// The screen's pages are `screen` now: where they are not the ones
    /// the pane last followed, the pane shows them, on the tab it showed
    /// where they have it, else on the first from its top.
    pub(crate) fn follow(&mut self, screen: Vec<&'static LearnPage>) {
        let same = screen.len() == self.followed.len()
            && screen
                .iter()
                .zip(&self.followed)
                .all(|(a, b)| std::ptr::eq(*a, *b));
        if same {
            return;
        }
        let on = self.state.pages.get(self.state.page).copied();
        let at = on.and_then(|p| screen.iter().position(|q| std::ptr::eq(*q, p)));
        if at.is_none() {
            self.state.scroll = 0.0;
        }
        self.state.page = at.unwrap_or(0);
        self.state.pages.clone_from(&screen);
        self.followed = screen;
    }

    /// Moves the sheet to `page`'s tab where it has one, else shows
    /// `page` alone.
    pub(crate) fn on(&mut self, page: &'static LearnPage, sheet: &mut Option<crate::Sheet>) {
        if let Some(i) = self.state.pages.iter().position(|p| std::ptr::eq(*p, page)) {
            self.state.page = i;
        } else {
            self.open(vec![page], sheet);
        }
    }

    /// Shows the sheet's page `i`.
    pub(crate) fn show_page(&mut self, i: u8) {
        if i == HOW_TAB {
            self.show_how();
        } else if usize::from(i) < self.state.pages.len() {
            self.state.page = usize::from(i);
            self.state.how = false;
            self.state.scroll = 0.0;
        }
    }

    /// The screen offers no blocks any more: the first page is on show.
    pub(crate) fn drop_how(&mut self) {
        self.state.how = false;
        self.state.scroll = 0.0;
    }

    /// Shows the How tab, from its top.
    pub(crate) fn show_how(&mut self) {
        if !self.state.how {
            self.state.how = true;
            self.state.scroll = 0.0;
        }
    }

    /// The steps ticked on `page` this session, in order.
    pub(crate) fn ticked(&self, page: &'static LearnPage) -> &[u32] {
        self.ticks
            .iter()
            .find(|(p, _)| std::ptr::eq(*p, page))
            .map_or(&[], |(_, d)| d.as_slice())
    }

    /// Ticks step `n` of the Learn sheet's page on show, or clears its
    /// tick if it is ticked already.
    pub(crate) fn tick_step(&mut self, n: u32) {
        let Some(page) = self.state.pages.get(self.state.page).copied() else {
            return;
        };
        let all = faraday_learn::steps(page);
        if !all.contains(&n) {
            return;
        }
        let mut done = self.ticked(page).to_vec();
        if let Some(at) = done.iter().position(|d| *d == n) {
            done.remove(at);
        } else {
            done.push(n);
        }
        let done: Vec<u32> = all.into_iter().filter(|s| done.contains(s)).collect();
        self.ticks.retain(|(p, _)| !std::ptr::eq(*p, page));
        if !done.is_empty() {
            self.ticks.push((page, done));
        }
    }
}

/// A `Screen` variant's name in a buffer on the stack, for the lookup of
/// its blocks (`faraday_learn::how`), which every frame asks several
/// times.
#[derive(Default)]
struct ScreenName {
    buf: [u8; 24],
    len: usize,
}

impl ScreenName {
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

impl std::fmt::Write for ScreenName {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        let room = self.buf.get_mut(self.len..end).ok_or(std::fmt::Error)?;
        room.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
