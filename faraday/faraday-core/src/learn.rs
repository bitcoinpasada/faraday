//! The Guide button: the guide's pages (`docs/guide/`, read through
//! `faraday_learn::guide`), opened where the person is
//! (`docs/GUIDE-IN-APP.md` §4). A screen and a step give the key of the
//! section the guide opens at ([`guide_key`]).

use faraday_learn::Block;

pub use faraday_learn::{GUIDES, MAKERS, Maker, guide_address, guide_id, maker};

use crate::{Action, Faraday, Screen};

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

/// The key of an Add a key form: `Part(i)` is the form at place `i` of
/// [`crate::forms::Form::ALL`], so a reordered list moves no key.
fn entry_key(at: GuideAt) -> &'static str {
    use crate::forms::Form;
    match at {
        GuideAt::Part(i) => match Form::ALL.get(usize::from(i)) {
            Some(Form::Words) => "entry.words",
            Some(Form::Slip39) => "entry.shares",
            Some(Form::Codex32) => "entry.codex32",
            Some(Form::Xor) => "entry.xor",
            None => "entry",
        },
        _ => "entry",
    }
}

/// The key of a Settings section: `Part(i)` is section `i` of Settings'
/// own list, which names each section's key, so a reordered list moves no
/// key.
fn settings_key(at: GuideAt) -> &'static str {
    match at {
        GuideAt::Part(i) => {
            crate::screens::settings_guide_key(usize::from(i)).unwrap_or("settings")
        }
        _ => "settings",
    }
}

/// The key of a vault's category: `Part(i)` is place `i` of
/// [`crate::vault::CATEGORIES`], found by the category's name, so a
/// reordered list moves no key.
fn vault_contents_key(at: GuideAt) -> &'static str {
    const KEYS: [(&str, &str); 6] = [
        (
            crate::strings::EN.vaults_bitcoin_keys,
            "vault-contents.bitcoin-keys",
        ),
        (crate::strings::EN.vaults_wallets, "vault-contents.wallets"),
        (crate::strings::EN.vaults_entries, "vault-contents.entries"),
        (crate::strings::EN.vaults_notes, "vault-contents.notes"),
        (
            crate::strings::EN.vaults_gpg_keys,
            "vault-contents.gpg-keys",
        ),
        (
            crate::strings::EN.vaults_secure_boot,
            "vault-contents.secure-boot",
        ),
    ];
    let GuideAt::Part(i) = at else {
        return "vault-contents";
    };
    crate::vault::CATEGORIES
        .get(usize::from(i))
        .and_then(|c| KEYS.iter().find(|k| k.0 == c.0))
        .map_or("vault-contents", |k| k.1)
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

/// The guide's state in the sheet and the pane: the place's key, the page
/// shown, whether the contents is, and how far it is scrolled.
#[derive(Default)]
pub struct LearnState {
    /// The key the guide was opened at, followed with the screen and the
    /// step ([`guide_key`]).
    pub key: &'static str,
    /// The place of the page on show; empty while the contents is.
    pub shown: &'static str,
    /// The contents is on show, in place of a page.
    pub contents: bool,
    /// The section the next drawing scrolls to the top, set when the
    /// guide opens, the step changes or a page opens: its offset needs
    /// the width the page is laid out at, so the drawing finds it and
    /// the pass after it ([`LearnArea::set_scroll`]) scrolls there.
    pub to: Option<&'static str>,
    /// Design units scrolled.
    pub scroll: f32,
    /// The furthest it can scroll, as last drawn.
    pub max: f32,
    /// The Cryptography tab is on show, in place of the page
    /// (`docs/CRYPTO-LEARN.md` §8.2). Kept only while the screen offers
    /// blocks ([`Faraday::learn_place`]).
    pub how: bool,
    /// The lines and heights of the page's texts at the width, font and
    /// scale they were last laid out at, so a frame drawn again breaks
    /// none of them ([`faraday_ui::roles::WrapBasis`]).
    wraps: std::cell::RefCell<Vec<GuideWrap>>,
}

/// The lines of a text as kept: none for a text of several runs.
pub type Lines = Option<std::rc::Rc<[(usize, usize)]>>;

/// One text of a guide page, broken into lines for a basis: the text by
/// where it is, what it was broken for, its height and its lines (none
/// for a text of several runs, whose height alone is kept).
pub struct GuideWrap {
    at: (usize, usize),
    basis: faraday_ui::roles::WrapBasis,
    h: f32,
    lines: Lines,
}

impl LearnState {
    /// The kept lines and height of the text at `at` for `basis`, if
    /// they are kept. A lookup.
    pub fn wrap_kept(
        &self,
        at: (usize, usize),
        basis: faraday_ui::roles::WrapBasis,
    ) -> Option<(f32, Lines)> {
        let kept = self.wraps.borrow();
        let e = kept.iter().find(|e| e.at == at && e.basis == basis)?;
        Some((e.h, e.lines.clone()))
    }

    /// Keeps the lines and height of the text at `at` for `basis`.
    pub fn wrap_keep(
        &self,
        at: (usize, usize),
        basis: faraday_ui::roles::WrapBasis,
        h: f32,
        lines: Lines,
    ) {
        let mut kept = self.wraps.borrow_mut();
        kept.retain(|e| e.at != at);
        kept.push(GuideWrap {
            at,
            basis,
            h,
            lines,
        });
    }
}

impl LearnState {
    /// The tab on show, for the fade between tabs: 0 for Cryptography, 2
    /// for the contents, else the page's place from 1. A lookup.
    pub fn tab(&self) -> usize {
        if self.how {
            0
        } else if self.contents {
            2
        } else {
            1
        }
    }

    /// The page of the place the guide was opened from, if one is
    /// written. A lookup.
    pub fn here(&self) -> Option<&'static faraday_learn::GuidePage> {
        faraday_learn::guide_page(place_of(self.key))
    }

    /// The page on show, if one is. A lookup.
    pub fn shown_page(&self) -> Option<&'static faraday_learn::GuidePage> {
        if self.contents || self.shown.is_empty() {
            return None;
        }
        faraday_learn::guide_page(self.shown)
    }

    /// Whether Back goes one step inside the guide before it closes: from
    /// a page opened from the contents, and from the contents to the
    /// place's page. A lookup.
    pub fn inside(&self) -> bool {
        if self.how {
            false
        } else if self.contents {
            self.here().is_some()
        } else {
            !self.shown.is_empty() && self.shown != place_of(self.key)
        }
    }
}

/// The place of a page, as it is kept in a key.
fn place_of_page(page: &'static faraday_learn::GuidePage) -> &'static str {
    page.place.as_str()
}

/// The place of a section key.
fn place_of(key: &'static str) -> &'static str {
    key.split('.').next().unwrap_or(key)
}

impl Faraday {
    /// The Guide button and the rail's and menu's Guide entry open the
    /// guide at the section of what is on screen, as the sheet or, on an
    /// extra-large window, the supporting pane beside the page; pressed
    /// again while the pane is shown, and the pane's close button, put the
    /// pane away; Done closes the sheet.
    pub(crate) fn learn_press(&mut self, action: crate::Action) {
        if action == crate::Action::How {
            self.how_press();
        } else if action == crate::Action::LearnClose || self.learn_pane_shown() {
            if self.sheet == Some(crate::Sheet::Learn) {
                self.sheet = None;
            }
            self.learn_close();
        } else {
            self.learn_open();
            self.learn_place();
        }
    }

    /// **Cryptography** on the bar: opens the guide on the Cryptography tab;
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

    /// The guide's scroll was set by a press, not by a person scrolling:
    /// the overlay scrollbar does not show for it.
    pub(crate) fn learn_bar_synced(&mut self) {
        let (key, _, moved_at) = self.scroll.bar_seen();
        self.scroll
            .set_bar_seen(key, self.learn.state().scroll, moved_at);
    }

    /// The key of what is on screen now ([`guide_key`]).
    fn screen_key(&self) -> &'static str {
        guide_key(self.screen, self.guide_here())
    }

    /// Opens the guide at the section of what is on screen.
    fn learn_open(&mut self) {
        let key = self.screen_key();
        self.learn.open_at(key, &mut self.sheet);
    }

    /// The pane's pin, which turns it on or off.
    pub(crate) fn learn_button(&mut self, action: crate::Action) {
        if action == crate::Action::LearnPin {
            self.learn.toggle_pin(self.screen);
        }
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
        // Back inside the guide goes a step before it closes.
        let open = self.sheet == Some(crate::Sheet::Learn)
            || (self.sheet.is_none() && self.learn_pane_shown());
        if open && self.learn.step_back() {
            self.learn_bar_synced();
            return;
        }
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

    /// Keeps the guide where the window and the screen put it, and the
    /// pane's page the screen's: called before every frame is drawn and
    /// after the guide opens. The sheet becomes the pane once the window
    /// holds one (extra-large), and the pane the sheet once it does not,
    /// unless another sheet is up; a pane on a screen that keeps the
    /// pane's place for its own goes, and so does an unpinned pane once
    /// the screen is another than the one it stood beside (owner,
    /// 2026-10-29). While the pane is open it follows the screen: a new
    /// step scrolls to its section, a new place opens its page; the
    /// contents stays where the person has it until then. Costs a lookup
    /// of the screen's key while the guide is open.
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
            if !self.learn_pane_place_free() {
                self.learn_close();
            } else {
                let now = self.screen_key();
                self.learn.follow(now);
            }
        } else if self.sheet == Some(crate::Sheet::Learn)
            && self.learn_pane_room()
            && self.learn_pane_place_free()
        {
            let now = self.screen_key();
            self.learn.move_to_pane(&mut self.sheet, now, self.screen);
        }
    }

    /// The Extra copies link: opens the guide at its section.
    pub(crate) fn learn_extra_copies(&mut self) {
        self.learn_open_key(EXTRA_COPIES_KEY);
    }

    /// The link to the shares page: the guide at the current place.
    pub(crate) fn learn_shares(&mut self) {
        self.learn_open_key(self.screen_key());
    }

    /// Opens the guide at `key`, where it is not open already; closing it
    /// leaves the screen as it was.
    fn learn_open_key(&mut self, key: &'static str) {
        if !self.learn.pane_open() {
            self.learn.open_at(key, &mut self.sheet);
        } else {
            self.learn.show_key(key);
        }
        self.learn_place();
    }

    /// The guide's state: the page on show and its scroll.
    pub fn learn(&self) -> &LearnState {
        self.learn.state()
    }

    /// Whether Learn's pane is pinned open across screens.
    pub fn learn_pinned(&self) -> bool {
        self.learn.pinned()
    }
}

/// The Learn sheet's area: its state.
#[derive(Default)]
pub(crate) struct LearnArea {
    state: LearnState,
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
    /// The screen's key as the pane last followed it. Kept in step with
    /// the screen by [`Faraday::learn_place`] before every frame: once
    /// the screen's key is another, the pane shows it. Empty while the
    /// pane is not open.
    followed: &'static str,
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

    /// Takes the extent and offset a frame drew the sheet with, and the
    /// offset of the section the guide was to scroll to, where it found
    /// it: the guide scrolls there and wants another frame.
    pub(crate) fn set_scroll(&mut self, max: f32, scroll: f32, section: Option<f32>) -> bool {
        self.state.max = max;
        self.state.scroll = scroll;
        if self.state.to.take().is_none() {
            return false;
        }
        self.state.scroll = section.unwrap_or(0.0).min(max);
        true
    }

    /// Opens the guide at `key`, as the sheet: the key's page, scrolled
    /// to its section; the contents where the place has no page.
    pub(crate) fn open_at(&mut self, key: &'static str, sheet: &mut Option<crate::Sheet>) {
        self.state = LearnState::default();
        self.show_key(key);
        self.close_pane();
        *sheet = Some(crate::Sheet::Learn);
    }

    /// Shows the page of `key`, scrolled to its section; the contents
    /// where the place has no page.
    pub(crate) fn show_key(&mut self, key: &'static str) {
        let st = &mut self.state;
        st.key = key;
        st.contents = faraday_learn::guide_page(place_of(key)).is_none();
        st.shown = place_of(key);
        st.to = (!st.contents).then_some(key);
        st.scroll = 0.0;
    }

    /// Back inside the guide: from a page opened from the contents to the
    /// contents, from the contents to the place's page. False where Back
    /// closes the guide.
    pub(crate) fn step_back(&mut self) -> bool {
        if !self.state.inside() {
            return false;
        }
        if self.state.contents {
            let key = self.state.key;
            self.show_key(key);
        } else {
            self.state.contents = true;
            self.state.scroll = 0.0;
        }
        true
    }

    /// Whether Learn is open as the supporting pane.
    pub(crate) fn pane_open(&self) -> bool {
        self.pane
    }

    /// The open sheet becomes the pane, on the page it shows, beside
    /// `on`; `screen` is the screen's key now, which the pane follows
    /// from here.
    pub(crate) fn move_to_pane(
        &mut self,
        sheet: &mut Option<crate::Sheet>,
        screen: &'static str,
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
        self.followed = "";
    }

    /// The screen's key is `screen` now: where it is not the one the pane
    /// last followed, the pane shows its page, scrolled to its section.
    pub(crate) fn follow(&mut self, screen: &'static str) {
        if screen == self.followed {
            return;
        }
        self.followed = screen;
        self.show_key(screen);
    }

    /// A press of a Learn action ([`crate::Action::LearnPage`] and the
    /// guide's own): a tab, **Contents**, the first row of the contents,
    /// or the page in place `i` of it.
    pub(crate) fn show_page(&mut self, action: Action) {
        let st = &mut self.state;
        match action {
            Action::LearnTabHow => self.show_how(),
            Action::LearnTabGuide => {
                st.how = false;
                st.scroll = 0.0;
                if !st.contents {
                    st.to = (st.shown == place_of(st.key)).then_some(st.key);
                }
            }
            Action::LearnContents => {
                st.how = false;
                st.contents = true;
                st.scroll = 0.0;
            }
            Action::LearnHere => {
                let key = st.key;
                st.how = false;
                self.show_key(key);
            }
            Action::LearnPage(n) => {
                let page = faraday_learn::guide_groups()
                    .iter()
                    .flat_map(|g| g.pages.iter())
                    .nth(usize::from(n));
                if let Some(page) = page {
                    st.how = false;
                    st.contents = false;
                    st.shown = place_of_page(page);
                    st.to = Some(place_of_page(page));
                    st.scroll = 0.0;
                }
            }
            _ => {}
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

impl Faraday {
    /// A lookup, made when the guide opens or the step changes; never
    /// while drawing. What the screen on show is at, for [`guide_key`]:
    /// the open card of a flow, a backup flow's kind and step, the sheet
    /// before its steps, a Settings section or a vault's category or an
    /// Add a key form (`docs/GUIDE-IN-APP.md` §1.2). The whole place
    /// where it holds nothing finer.
    pub(crate) fn guide_here(&self) -> GuideAt {
        use crate::trail::StepFlow;
        let open = |flow: &dyn StepFlow| flow.steps(self).and_then(|s| s.open);
        let step = |n: Option<u8>| n.map_or(GuideAt::Whole, GuideAt::Step);
        match self.screen {
            Screen::Create => step(self.create.state().and_then(|c| c.open)),
            Screen::Restore => step(open(&crate::restore::RESTORE_STEPS)),
            Screen::Spend => step(open(&crate::wallet::SPEND_STEPS)),
            Screen::Message => step(open(&crate::screens::MESSAGE_STEPS)),
            Screen::Bip85 => step(open(&crate::bip85::BIP85_STEPS)),
            Screen::Convert => step(open(&crate::convert::CONVERT_STEPS)),
            Screen::KefTool => step(open(&crate::kef_tool::KEF_TOOL_STEPS)),
            Screen::CoreWallet => step(open(&crate::core_wallet::tool::CORE_WALLET_STEPS)),
            Screen::Vanity => step(open(&crate::vanity::VANITY_STEPS)),
            Screen::KeyGen => step(self.keygen.state().and_then(|k| k.open)),
            Screen::QrTool
                if crate::qr_tool::state(self).is_some_and(|s| s.code_to_open().is_some()) =>
            {
                GuideAt::Opening
            }
            Screen::QrTool => step(open(&crate::qr_tool::QR_TOOL_STEPS)),
            Screen::Backup => match self.backup.state() {
                Some(b) if b.stage == crate::BStage::Checklist => {
                    b.open.map_or(GuideAt::Whole, GuideAt::Item)
                }
                Some(b) => step(b.q),
                None => GuideAt::Whole,
            },
            Screen::SpendBackup if self.spend_backup_overview() => GuideAt::Whole,
            Screen::SpendBackup => match self.spend_backup_open_card() {
                Some(crate::spend_backup::CardId::Page(p)) => GuideAt::Step(p),
                Some(crate::spend_backup::CardId::Step(n)) => GuideAt::Inner(n),
                _ => GuideAt::Whole,
            },
            Screen::BackupFlow if self.sheet == Some(crate::Sheet::GpgRestore) => GuideAt::Sheet,
            Screen::BackupFlow => match self.flow() {
                Some(f)
                    if f.restore.is_some()
                        && f.live.open == Some(crate::backup_flow::gpg_restore::HEAD) =>
                {
                    GuideAt::Head
                }
                Some(f) => match f
                    .live
                    .open
                    .and_then(|n| crate::backup_flow::Step::ALL.get(usize::from(n)))
                {
                    Some(&s) => GuideAt::Flow(f.kind, s),
                    None => GuideAt::Kind(f.kind),
                },
                None => GuideAt::Whole,
            },
            Screen::Entry => crate::forms::Form::ALL
                .iter()
                .position(|&f| f == self.entry.state().form)
                .map_or(GuideAt::Whole, |i| GuideAt::Part(i as u8)),
            Screen::VaultContents => GuideAt::Part(self.vaults.category as u8),
            Screen::Settings if crate::panes::settings_listed(self.window()) => self
                .scroll
                .panes()
                .settings()
                .map_or(GuideAt::Whole, GuideAt::Part),
            _ => GuideAt::Whole,
        }
    }
}

/// The guide opens at the step the person is on (`docs/GUIDE-IN-APP.md`
/// §1.2).
#[cfg(test)]
mod at_tests {
    use super::{GuideAt, guide_key};
    use crate::{Action, Screen, testkit};

    fn key(app: &crate::Faraday) -> &'static str {
        guide_key(app.screen, app.guide_here())
    }

    #[test]
    fn a_new_wallets_open_step_names_its_section() {
        let mut app = testkit::started();
        app.press(Action::CreateWallet);
        assert_eq!(app.screen, Screen::Create);
        app.press(Action::CStep(crate::cstep::WHERE));
        assert_eq!(key(&app), "create.where");
        app.press(Action::CStep(crate::cstep::KIND));
        assert_eq!(key(&app), "create.kind");
    }

    #[test]
    fn add_a_key_names_the_form_chosen() {
        let mut app = testkit::started();
        app.press(Action::Nav(Screen::Entry));
        assert_eq!(app.screen, Screen::Entry);
        let at = crate::forms::Form::ALL
            .iter()
            .position(|f| *f == crate::forms::Form::Codex32)
            .unwrap();
        app.press(Action::EntryForm(at as u8));
        assert_eq!(key(&app), "entry.codex32");
    }

    #[test]
    fn a_vault_names_the_category_open() {
        let mut app = testkit::started();
        app.screen = Screen::VaultContents;
        app.vaults.set_category(4);
        assert_eq!(key(&app), "vault-contents.gpg-keys");
        app.vaults.set_category(5);
        assert_eq!(key(&app), "vault-contents.secure-boot");
    }

    #[test]
    fn compact_settings_names_the_section_open_and_the_list_its_page() {
        use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
        let mut app = testkit::started();
        app.event(Event::Display(DisplayInfo {
            width: 480,
            height: 800,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        }));
        app.press(Action::Nav(Screen::Settings));
        assert_eq!(app.guide_here(), GuideAt::Whole);
        assert_eq!(key(&app), "settings");
        app.press(Action::SettingsSection(Some(2)));
        assert_eq!(key(&app), "settings.time");
    }
}
