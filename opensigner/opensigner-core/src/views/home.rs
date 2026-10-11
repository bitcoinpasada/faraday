//! Home (`docs/DESIGN.md` §5 Hub) and the tiers Document behind the tier
//! badge.
//!
//! Home is the launcher: the status line, then the six tiles — Wallets,
//! Keys, Scan, Tools, Learn, Settings — and nothing else. It never
//! changes shape, whatever is loaded (`docs/PLANNING.md` §16.104 rule
//! 4). What is loaded is on Wallets and on Keys, which carry their own
//! empty states.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::policy::{Template, WalletPolicy};
use osk_ui::geom::SizeClass;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Row, Section};
use osk_ui::widgets::{Icon, Tone};

use crate::{AssuranceTier, OpenSigner, hub_tiles, ids, strings, text};

impl OpenSigner {
    /// §5 Hub: the status line and the six tiles.
    pub(crate) fn view_home(&self) -> Node {
        let s = self.strings();
        self.with_chrome(None, |c| screens::launcher(c, &self.status(), hub_tiles(s)))
    }

    /// §4.1 Status line: the tier badge, the network badge off mainnet,
    /// the session badge while this device holds a MuSig2 secret nonce,
    /// and the lock icon button.
    ///
    /// §4.8 gives the badge its full form, "Tier C · desktop", and the
    /// short one only on the 268 dp line, where the full form does not
    /// fit beside the network badge and the two buttons.
    fn status(&self) -> screens::Status {
        let s = self.strings();
        let session = self
            .musig_session_open()
            .then(|| String::from(s.status_session_open));
        let tier = match (self.class(), session.is_some()) {
            // §4.8: the short form on the 268 dp line, and the letter
            // alone while the session badge stands beside it.
            (SizeClass::Small, true) => self.tier.badge_letter(s),
            (SizeClass::Small, false) => self.tier.badge_short(s),
            _ => self.tier.badge(s),
        };
        screens::Status {
            tier: String::from(tier),
            tier_id: Some(ids::STATUS_TIER),
            network: text::network(self.network),
            // There is something to lock only once a key is in memory
            // and the session has a PIN to unlock it with.
            lock: (self.has_secrets() && self.has_pin()).then_some(ids::STATUS_LOCK),
            session,
            notice: self.leave_notice(),
        }
    }

    /// §4.4 glyph: the wallet when this device can sign for it, the eye
    /// when it can only watch it. The key glyph is never used here, so
    /// that the fingerprint glyph a key row carries means one thing
    /// (`docs/PLANNING.md` §16.127 rule 2).
    pub(crate) fn wallet_glyph(&self, holds_key: bool) -> Icon {
        if holds_key { Icon::Wallet } else { Icon::Eye }
    }

    /// Whether one of this device's keys is a key of `policy`, which is
    /// what the key glyph says. A key with no origin names no master, so
    /// it is nobody's.
    pub(crate) fn wallet_holds_key(&self, policy: &WalletPolicy) -> bool {
        // A FROST wallet's members are public shares, and the key glyph
        // says one of this device's keys computes one of them
        // (`docs/PLANNING.md` §16.104 rule 3).
        if let Some(record) = policy.record() {
            return self
                .keys
                .iter()
                .filter_map(|k| k.share)
                .any(|p| record.info.pubshares.contains(&Some(p)));
        }
        // §16.113: a silent payments wallet names its key by the origin
        // the record states, since the wallet is two public keys and no
        // key expression.
        if let Some(record) = policy.silent() {
            return self
                .keys
                .iter()
                .any(|k| k.fingerprint == record.fingerprint && k.network == record.network);
        }
        policy.keys().iter().any(|pk| {
            pk.fingerprint()
                .is_some_and(|fp| self.keys.iter().any(|k| k.fingerprint == fp))
        })
    }

    /// "multisig · regtest": what a policy is, and the chain it pays
    /// on, which is the device's own.
    pub(crate) fn policy_subtitle(&self, policy: &WalletPolicy) -> String {
        let s = self.strings();
        let recovery = osk_bip::recovery::RecoveryPolicy::from_wallet_policy(policy).is_some();
        let kind = match (policy.template(), policy.quorum()) {
            _ if policy.silent().is_some() => s.wallet_kind_silent,
            _ if recovery => s.wallet_kind_recovery,
            _ if policy.tapscript_quorum().is_some() => s.wallet_kind_taproot_multisig,
            (_, Some(_)) => s.wallet_kind_multisig,
            (Template::MuSig, _) => s.wallet_musig,
            (Template::Threshold { .. }, _) => s.wallet_kind_threshold,
            (Template::Miniscript { .. }, _) => s.wallet_kind_miniscript,
            (Template::Tree, _) => s.wallet_kind_tree,
            _ => s.wallet_single,
        };
        alloc::format!("{kind} \u{00b7} {}", self.network.name())
    }

    /// §5 Menu, "Add a key": the ways a key is made (§16.104 rule 1,
    /// §16.107 rule 4). Opening one key from another is done on that
    /// key's own page, so it is not here (§16.127 rule 3) — except
    /// while a wallet's or a transaction's row is asking for one key in
    /// particular, where the key asked for may be behind a passphrase
    /// or a BIP-85 index and the screen must offer the two ways to it
    /// (§16.104 rule 6). They ask which loaded key to start from.
    pub(crate) fn view_add(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let mut rows = vec![
                Self::add_row(ids::KEYS_LOAD, Icon::Download, s.home_load_key),
                Self::add_row(ids::KEYS_CREATE, Icon::Dice, s.home_create_key),
                // Making words and making shares are two flows, never
                // one screen asking which (§16.107 rule 4).
                Self::add_row(ids::ADD_CREATE_SLIP39, Icon::Scissors, s.add_create_slip39),
                Self::add_row(ids::ADD_CREATE_CODEX32, Icon::Numbers, s.add_create_codex32),
            ];
            if self.key_was_asked_for() {
                if self.passphrase_sources().next().is_some() {
                    rows.push(Self::add_row(
                        ids::ADD_OPEN_PASSPHRASE,
                        Icon::Passphrase,
                        s.detail_open_passphrase,
                    ));
                }
                if !self.keys.is_empty() {
                    rows.push(Self::add_row(
                        ids::ADD_OPEN_CHILD,
                        Icon::Derivation,
                        s.detail_open_child,
                    ));
                }
            }
            screens::menu(c, s.keys_add, None, rows, Vec::new())
        })
    }

    /// §5 Menu, "Add a wallet": the one wizard that builds every kind,
    /// and the scanner that reads one from outside (§16.104 rule 2).
    pub(crate) fn view_add_wallet(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let rows = vec![
                Self::add_row(ids::BUILD_NEW, Icon::Wallet, s.build_new),
                Self::add_row(ids::WALLETS_LOAD, Icon::Scan, s.wallet_load),
            ];
            screens::menu(c, s.wallets_add, None, rows, Vec::new())
        })
    }

    /// One row of either Add menu.
    fn add_row(id: ids::Id, icon: Icon, label: &str) -> Row {
        Row::Menu {
            id,
            icon: Some(icon),
            label: String::from(label),
            value: None,
            tone: Tone::Text,
        }
    }

    /// §5 Document, "Tiers": what the tiers mean, this device's own
    /// first. Reached from the tier badge in the status line and from
    /// About. §4.12 allows the prose here: a Document is one of the two
    /// places a sentence belongs.
    pub(crate) fn view_tiers(&self) -> Node {
        let s = self.strings();
        // The one fact the screen is opened for comes first, so it is
        // never the clipped last line (UX review 2026-09-07, §3.1).
        let mut sections = vec![Section {
            id: None,
            heading: strings::fill1(s.tier_this_device, self.tier.name(s)),
            paragraphs: vec![String::from(self.tier.statement(s))],
        }];
        for tier in AssuranceTier::ALL.iter().filter(|t| **t != self.tier) {
            sections.push(Section {
                id: None,
                heading: String::from(tier.name(s)),
                paragraphs: vec![String::from(tier.statement(s))],
            });
        }
        self.with_chrome(Some(ids::BACK), |c| {
            screens::document(c, s.tier_title, sections)
        })
    }
}
