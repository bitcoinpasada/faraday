//! The ? button: OpenSigner's Learn pages, read from OpenSigner's own
//! strings (`opensigner_core::strings::EN`, edited as `docs/learn/*.md`
//! upstream), so Faraday shows the same text and never a copy that
//! drifts. Each screen names the pages that explain it; a wallet's screens
//! name the pages for its kind.

use opensigner_core::strings::{EN, LearnPage};

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
        Kind::Single(_) => vec![&EN.learn_wallet_kinds, &EN.learn_words],
        Kind::Multi(_) => vec![&EN.learn_multisig, &EN.learn_xpubs, &EN.learn_coordinators],
        Kind::TapMulti => vec![&EN.learn_multisig, &EN.learn_spend_paths],
        Kind::Miniscript | Kind::Tree => {
            vec![
                &EN.learn_spend_paths,
                &EN.learn_inheritance,
                &EN.learn_wallet_kinds,
            ]
        }
        Kind::MuSig => vec![&EN.learn_nonces, &EN.learn_multisig],
        Kind::Threshold => vec![&EN.learn_frost, &EN.learn_nonces],
        Kind::Other => vec![&EN.learn_wallet_kinds],
    }
}

fn for_new(kind: NewKind) -> Vec<&'static LearnPage> {
    match kind {
        NewKind::NativeSegwit | NewKind::Taproot | NewKind::NestedSegwit | NewKind::Legacy => {
            vec![&EN.learn_wallet_kinds, &EN.learn_words, &EN.learn_backups]
        }
        NewKind::Multi | NewKind::MultiNested | NewKind::MultiLegacy => {
            vec![&EN.learn_multisig, &EN.learn_xpubs, &EN.learn_coordinators]
        }
        NewKind::TapMulti => vec![&EN.learn_multisig, &EN.learn_spend_paths],
        NewKind::Threshold => vec![&EN.learn_frost, &EN.learn_nonces],
        NewKind::MuSig => vec![&EN.learn_nonces, &EN.learn_wallet_kinds],
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
            Screen::Home => vec![&EN.learn_start_here, &EN.learn_air_gap],
            Screen::Family => vec![
                &EN.learn_inheritance,
                &EN.learn_transactions,
                &EN.learn_air_gap,
                &EN.learn_scams,
            ],
            Screen::Start => vec![&EN.learn_wallet_kinds, &EN.learn_start_here],
            Screen::Wallets => wallet().map_or_else(|| vec![&EN.learn_wallet_kinds], for_kind),
            Screen::Spend => {
                let mut p = vec![&EN.learn_transactions, &EN.learn_verifying];
                if let Some(k) = wallet()
                    && matches!(k, Kind::MuSig | Kind::Threshold)
                {
                    p.push(&EN.learn_nonces);
                }
                p.push(&EN.learn_scams);
                p
            }
            Screen::Create => self
                .create
                .as_ref()
                .map_or_else(|| vec![&EN.learn_wallet_kinds], |c| for_new(c.kind)),
            Screen::KeyGen => vec![
                &EN.learn_randomness,
                &EN.learn_where_randomness,
                &EN.learn_words,
            ],
            Screen::Entry => vec![&EN.learn_words, &EN.learn_passphrases],
            Screen::Silent => vec![&EN.learn_silent_payments, &EN.learn_xpubs],
            Screen::Explore => vec![&EN.learn_xpubs, &EN.learn_glossary],
            Screen::Vanity => vec![&EN.learn_passphrases, &EN.learn_wallet_kinds],
            Screen::Lightning => vec![&EN.learn_glossary],
            Screen::Tools => vec![&EN.learn_tools, &EN.learn_glossary],
            Screen::Bip85 => vec![&EN.learn_passphrases, &EN.learn_glossary],
            Screen::Backup => vec![
                &EN.learn_backups,
                &EN.learn_other_backups,
                &EN.learn_seed_xor,
            ],
            Screen::Restore => vec![&EN.learn_backups, &EN.learn_inheritance],
            Screen::Message | Screen::CheckMessage => vec![&EN.learn_message],
            Screen::Files | Screen::Visit => vec![&EN.learn_air_gap, &EN.learn_coordinators],
            Screen::Vaults | Screen::CreateVault | Screen::Unlock | Screen::VaultContents => {
                vec![&EN.learn_encrypted_backups, &EN.learn_passphrases]
            }
            Screen::Settings => vec![&EN.learn_secure_element, &EN.learn_glossary],
            Screen::Decode => vec![&EN.learn_transactions, &EN.learn_verifying],
            Screen::Catalog => vec![&EN.learn_tools, &EN.learn_glossary],
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
