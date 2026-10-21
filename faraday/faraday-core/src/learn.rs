//! The ? button: the Learn pages, read from `faraday_learn::pages::EN`
//! (edited as `docs/learn/*.md`), so Faraday shows the same text
//! and never a copy that
//! drifts. Each screen names the pages that explain it; a wallet's screens
//! name the pages for its kind.

use faraday_learn::Page as LearnPage;
use faraday_learn::pages::EN;

use faraday_learn::known;
pub use faraday_learn::{
    GUIDES, MAKERS, Maker, apk_key_page, backup_plan_check_page, demo_page, gpg_page,
    guide_address, guide_id, keys_kept_page, maker, page, shares_page, time_page, upgrade_page,
    vaults_page,
};

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
    /// The Learn pages that explain what is on screen. Where the demo is
    /// set up (F20), the screens where keys come in and the network is
    /// chosen name its page first, and so, while the demo is on (on
    /// mainnet, where its actions are dimmed), does every screen with an
    /// action the demo dims (`docs/DEMO.md` §4, §7).
    pub fn learn_pages(&self) -> Vec<&'static LearnPage> {
        let mut pages = self.screen_pages();
        let keys_in = self.session.has_demo()
            && matches!(
                self.screen,
                Screen::Home | Screen::Entry | Screen::KeyGen | Screen::Settings
            );
        let dims = self.session.demo()
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
                .get(self.wallet)
                .map(|w| Kind::of(w.policy()))
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
                    vec![
                        &EN.backups,
                        backup_plan_check_page(),
                        shares_page(),
                        &EN.other_backups,
                        &EN.seed_xor,
                    ]
                } else {
                    vec![
                        &EN.backups,
                        backup_plan_check_page(),
                        &EN.other_backups,
                        &EN.seed_xor,
                    ]
                }
            }
            Screen::Backups => vec![&EN.backups, backup_plan_check_page(), vaults_page()],
            Screen::Restore => vec![&EN.backups, backup_plan_check_page(), &EN.inheritance],
            Screen::Message | Screen::CheckMessage => vec![&EN.message],
            Screen::Files | Screen::Visit => vec![&EN.air_gap, &EN.coordinators],
            // An APK signing key's page, and the form that makes one, name
            // the page about it (`docs/APK-SIGNING.md` §10).
            // An OpenSigner backup being read into the vault names the
            // page about OpenSigner's backups first.
            Screen::VaultContents
                if self
                    .vaults
                    .prompt
                    .as_ref()
                    .is_some_and(|p| matches!(p.purpose, crate::vaults::Purpose::Backup(_))) =>
            {
                vec![&EN.encrypted_backups, vaults_page(), &EN.passphrases]
            }
            Screen::VaultContents if self.vaults.category == 6 => {
                vec![apk_key_page(), vaults_page(), &EN.passphrases]
            }
            Screen::VaultContents if self.vaults.category == 4 => {
                vec![gpg_page(), vaults_page(), &EN.passphrases]
            }
            Screen::Vaults
            | Screen::CreateVault
            | Screen::Unlock
            | Screen::BackupFlow
            | Screen::VaultContents => {
                vec![vaults_page(), &EN.passphrases]
            }
            Screen::Settings if self.online => vec![keys_kept_page(), &EN.glossary],
            Screen::Settings => vec![keys_kept_page(), &EN.glossary, upgrade_page()],
            Screen::SetTime | Screen::Codes => vec![time_page()],
            // A stick can also be replaced from a computer, keeping its
            // data (`docs/STARTER.md` §4.4); a Pi card cannot yet (§3.3).
            Screen::Upgrade if self.medium == crate::medium::Medium::Stick => {
                vec![upgrade_page(), known("replace-faraday")]
            }
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
        self.learn_on(shares_page());
    }

    /// The Learn sheet on `page`: on its tab where the screen's pages
    /// have it, else on it alone.
    pub(crate) fn learn_on(&mut self, page: &'static LearnPage) {
        self.learn_open();
        if let Some(i) = self.learn.pages.iter().position(|p| std::ptr::eq(*p, page)) {
            self.learn.page = i;
        } else {
            self.learn = LearnState {
                pages: vec![page],
                ..LearnState::default()
            };
            self.sheet = Some(crate::Sheet::Learn);
        }
    }

    /// Opens the Learn sheet on `pages` over the screen on show: for the
    /// app holding this one ([`Faraday::held`]), whose own screens name
    /// the pages. Closing the sheet leaves the screen as it was.
    pub fn learn_with(&mut self, pages: Vec<&'static LearnPage>) {
        if pages.is_empty() {
            return;
        }
        self.learn = LearnState {
            pages,
            ..LearnState::default()
        };
        self.sheet = Some(crate::Sheet::Learn);
        self.dirty = true;
        self.commands.push_back(faraday_shell_api::Command::Draw);
    }

    /// Shows the sheet's page `i`.
    pub(crate) fn learn_page(&mut self, i: u8) {
        if usize::from(i) < self.learn.pages.len() {
            self.learn.page = usize::from(i);
            self.learn.scroll = 0.0;
        }
    }

    /// The steps ticked on `page` this session, in order.
    /// Kept in memory only: leaving and
    /// reopening the page in the same session keeps them; a new session
    /// starts unticked.
    pub fn learn_ticked(&self, page: &'static LearnPage) -> &[u32] {
        self.learn_ticks
            .iter()
            .find(|(p, _)| std::ptr::eq(*p, page))
            .map_or(&[], |(_, d)| d.as_slice())
    }

    /// Ticks step `n` of the Learn sheet's page on show, or clears its
    /// tick if it is ticked already.
    pub(crate) fn learn_tick_step(&mut self, n: u32) {
        let Some(page) = self.learn.pages.get(self.learn.page).copied() else {
            return;
        };
        let all = faraday_learn::steps(page);
        if !all.contains(&n) {
            return;
        }
        let mut done = self.learn_ticked(page).to_vec();
        if let Some(at) = done.iter().position(|d| *d == n) {
            done.remove(at);
        } else {
            done.push(n);
        }
        let done: Vec<u32> = all.into_iter().filter(|s| done.contains(s)).collect();
        self.learn_ticks.retain(|(p, _)| !std::ptr::eq(*p, page));
        if !done.is_empty() {
            self.learn_ticks.push((page, done));
        }
    }
}
