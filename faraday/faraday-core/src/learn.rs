//! The ? button: OpenSigner's Learn pages, read from `osk_learn::EN`
//! (edited as `docs/learn/*.md` upstream), so Faraday shows the same text
//! and never a copy that
//! drifts. Each screen names the pages that explain it; a wallet's screens
//! name the pages for its kind.

use osk_learn::{EN, Page as LearnPage};

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
            Screen::Backup => vec![&EN.backups, &EN.other_backups, &EN.seed_xor],
            Screen::Restore => vec![&EN.backups, &EN.inheritance],
            Screen::Message | Screen::CheckMessage => vec![&EN.message],
            Screen::Files | Screen::Visit => vec![&EN.air_gap, &EN.coordinators],
            Screen::Vaults | Screen::CreateVault | Screen::Unlock | Screen::VaultContents => {
                vec![&EN.encrypted_backups, &EN.passphrases]
            }
            Screen::Settings => vec![&EN.secure_element, &EN.glossary],
            Screen::Decode => vec![&EN.transactions, &EN.verifying],
            Screen::Catalog => vec![&EN.tools, &EN.glossary],
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

    /// Shows the sheet's page `i`.
    pub(crate) fn learn_page(&mut self, i: u8) {
        if usize::from(i) < self.learn.pages.len() {
            self.learn.page = usize::from(i);
            self.learn.scroll = 0.0;
        }
    }
}
