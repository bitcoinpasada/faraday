//! The ? button: OpenSigner's Learn pages, read from `osk_learn::EN`
//! (edited as `docs/learn/*.md` upstream), so Faraday shows the same text
//! and never a copy that
//! drifts. Each screen names the pages that explain it; a wallet's screens
//! name the pages for its kind.

use std::sync::OnceLock;

use osk_learn::{EN, Page as LearnPage, Section};

use crate::create::NewKind;
use crate::wallet::Kind;
use crate::{Faraday, Screen};

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
}

/// Faraday's own page on upgrading a stick (`PLAN.md` §5.5), which
/// OpenSigner does not have. It is edited as
/// `docs/learn/faraday/upgrade-a-stick.md`, beside OpenSigner's pages but
/// outside the set `tools/learn/sync.py` keeps in step with
/// `osk_learn::EN`, and read from that file when Faraday is built.
const UPGRADE_MD: &str = include_str!("../../../docs/learn/faraday/upgrade-a-stick.md");

/// Faraday's own page on a multisig backup's shares
/// (`docs/NEW-WALLET.md` §4.2), edited as `docs/learn/faraday/shares.md`
/// and read as [`UPGRADE_MD`] is.
const SHARES_MD: &str = include_str!("../../../docs/learn/faraday/shares.md");

/// A page from its Markdown, as `docs/learn/` writes pages: `# ` the
/// title, `## ` a section, paragraphs separated by blank lines.
fn page_of(md: &'static str) -> LearnPage {
    let mut title = "";
    let mut sections: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    let mut para: Vec<&'static str> = Vec::new();
    let flush = |para: &mut Vec<&'static str>,
                 sections: &mut Vec<(&'static str, Vec<&'static str>)>| {
        if para.is_empty() {
            return;
        }
        let text: &'static str = Box::leak(para.join(" ").into_boxed_str());
        para.clear();
        if let Some(s) = sections.last_mut() {
            s.1.push(text);
        }
    };
    for line in md.lines() {
        if let Some(t) = line.strip_prefix("# ") {
            title = t;
        } else if let Some(h) = line.strip_prefix("## ") {
            flush(&mut para, &mut sections);
            sections.push((h, Vec::new()));
        } else if line.trim().is_empty() {
            flush(&mut para, &mut sections);
        } else {
            para.push(line.trim());
        }
    }
    flush(&mut para, &mut sections);
    let sections: Vec<Section> = sections
        .into_iter()
        .map(|(heading, paragraphs)| Section {
            heading,
            paragraphs: Box::leak(paragraphs.into_boxed_slice()),
        })
        .collect();
    LearnPage {
        title,
        sections: Box::leak(sections.into_boxed_slice()),
    }
}

/// "Upgrading a Faraday stick", read once.
pub fn upgrade_page() -> &'static LearnPage {
    static PAGE: OnceLock<LearnPage> = OnceLock::new();
    PAGE.get_or_init(|| page_of(UPGRADE_MD))
}

/// "Shares of a wallet description", read once.
pub fn shares_page() -> &'static LearnPage {
    static PAGE: OnceLock<LearnPage> = OnceLock::new();
    PAGE.get_or_init(|| page_of(SHARES_MD))
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
        Kind::Threshold => vec![&EN.frost, &EN.nonces],
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
        NewKind::Threshold => vec![&EN.frost, &EN.nonces],
        NewKind::MuSig => vec![&EN.nonces, &EN.wallet_kinds],
    }
}

impl Faraday {
    /// The Learn pages that explain what is on screen.
    pub fn learn_pages(&self) -> Vec<&'static LearnPage> {
        let wallet = || {
            self.session
                .wallets
                .get(self.wallet)
                .map(|w| Kind::of(&w.policy))
        };
        match self.screen {
            Screen::Home => vec![&EN.start_here, &EN.air_gap],
            Screen::Family => vec![&EN.inheritance, &EN.transactions, &EN.air_gap, &EN.scams],
            Screen::Start => vec![&EN.wallet_kinds, &EN.start_here],
            Screen::Wallets => wallet().map_or_else(|| vec![&EN.wallet_kinds], for_kind),
            Screen::Spend => {
                let mut p = vec![&EN.transactions, &EN.verifying];
                if let Some(k) = wallet()
                    && matches!(k, Kind::MuSig | Kind::Threshold)
                {
                    p.push(&EN.nonces);
                }
                p.push(&EN.scams);
                p
            }
            Screen::Create => self
                .create
                .as_ref()
                .map_or_else(|| vec![&EN.wallet_kinds], |c| for_new(c.kind)),
            Screen::KeyGen => vec![&EN.randomness, &EN.where_randomness, &EN.words],
            Screen::Entry => vec![&EN.words, &EN.passphrases],
            Screen::Silent => vec![&EN.silent_payments, &EN.xpubs],
            Screen::Explore => vec![&EN.xpubs, &EN.glossary],
            Screen::Vanity => vec![&EN.passphrases, &EN.wallet_kinds],
            Screen::Lightning => vec![&EN.glossary],
            Screen::Tools => vec![&EN.tools, &EN.glossary],
            Screen::Bip85 => vec![&EN.passphrases, &EN.glossary],
            Screen::Backup => {
                // A multisig's backup can keep shares of its description.
                let splits = self
                    .backup
                    .as_ref()
                    .and_then(|b| self.session.wallets.get(b.wallet))
                    .is_some_and(crate::backup::splits);
                if splits {
                    vec![&EN.backups, shares_page(), &EN.other_backups, &EN.seed_xor]
                } else {
                    vec![&EN.backups, &EN.other_backups, &EN.seed_xor]
                }
            }
            Screen::Backups => vec![&EN.backups, &EN.encrypted_backups],
            Screen::Restore => vec![&EN.backups, &EN.inheritance],
            Screen::Message | Screen::CheckMessage => vec![&EN.message],
            Screen::Files | Screen::Visit => vec![&EN.air_gap, &EN.coordinators],
            Screen::Vaults | Screen::CreateVault | Screen::Unlock | Screen::VaultContents => {
                vec![&EN.encrypted_backups, &EN.passphrases]
            }
            Screen::Settings if self.online => vec![&EN.secure_element, &EN.glossary],
            Screen::Settings => vec![&EN.secure_element, &EN.glossary, upgrade_page()],
            Screen::Upgrade => vec![upgrade_page()],
            Screen::Decode => vec![&EN.transactions, &EN.verifying],
            Screen::Catalog => vec![&EN.tools, &EN.glossary],
            Screen::Transfer => vec![&EN.air_gap, &EN.coordinators],
        }
    }

    /// Opens the Learn sheet on the screen's first page.
    pub(crate) fn learn_open(&mut self) {
        let pages = self.learn_pages();
        if pages.is_empty() {
            return;
        }
        self.learn = LearnState {
            pages,
            ..LearnState::default()
        };
        self.sheet = Some(crate::Sheet::Learn);
    }

    /// Opens the Learn sheet on the page about shares; closing it leaves
    /// the screen as it was.
    pub(crate) fn learn_shares(&mut self) {
        self.learn_open();
        if let Some(i) = self
            .learn
            .pages
            .iter()
            .position(|p| std::ptr::eq(*p, shares_page()))
        {
            self.learn.page = i;
        } else {
            self.learn = LearnState {
                pages: vec![shares_page()],
                ..LearnState::default()
            };
            self.sheet = Some(crate::Sheet::Learn);
        }
    }

    /// Shows the sheet's page `i`.
    pub(crate) fn learn_page(&mut self, i: u8) {
        if usize::from(i) < self.learn.pages.len() {
            self.learn.page = usize::from(i);
            self.learn.scroll = 0.0;
        }
    }
}
