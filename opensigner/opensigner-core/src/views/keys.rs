//! Keys and Wallets, the two lists the launcher opens
//! (`docs/PLANNING.md` §16.104 rules 1 and 2).
//!
//! Keys is one row per loaded key: the fingerprint glyph, the
//! fingerprint and what the key is made of (§16.127 rule 1). Wallets is
//! a list of what was registered or built here, each row with the wallet
//! glyph or the eye. Both carry §4.14's empty state and a bottom action
//! that opens their Add menu.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Row};
use osk_ui::widgets::{Icon, Tone};

use crate::{OpenSigner, PickFor, WalletRef, ids, text};

impl OpenSigner {
    /// §5 Menu, "Keys": one row per loaded key, in the order they were
    /// loaded. A row carries the fingerprint and what the key is made
    /// of, and never the network, which is the device's and not the
    /// key's (§16.127 rule 1).
    pub(crate) fn view_keys(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let mut rows: Vec<Row> = self
                .keys
                .iter()
                .enumerate()
                .map(|(i, k)| Row::Key {
                    id: ids::at(ids::KEYS_ROW_BASE, i),
                    fingerprint: text::fingerprint_hex(k.fingerprint),
                    subtitle: self.key_made_of(i),
                })
                .collect();
            // §4.14: with nothing loaded the list is the rows that start
            // the two flows, never a sentence about emptiness.
            if rows.is_empty() {
                rows = vec![
                    Row::Action {
                        id: ids::KEYS_LOAD,
                        icon: Icon::Download,
                        label: String::from(s.home_load_key),
                    },
                    Row::Action {
                        id: ids::KEYS_CREATE,
                        icon: Icon::Dice,
                        label: String::from(s.home_create_key),
                    },
                ];
            }
            screens::menu(
                c,
                s.home_keys,
                None,
                rows,
                vec![Action::new(ids::KEYS_ADD, s.keys_add)],
            )
        })
    }

    /// §5 Menu, "Wallets": one row per registered policy, with the glyph
    /// that says whether this device can sign for it.
    pub(crate) fn view_wallets(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let mut rows: Vec<Row> = self
                .wallets
                .iter()
                .enumerate()
                .map(|(i, policy)| Row::MonoMenu {
                    id: ids::at(ids::WALLETS_POLICY_ROW_BASE, i),
                    icon: Some(self.wallet_glyph(self.wallet_holds_key(policy))),
                    label: self.policy_subtitle(policy),
                    value: self.wallet_call_name(WalletRef::Policy(i)),
                })
                .collect();
            if rows.is_empty() {
                rows = vec![
                    Row::Action {
                        id: ids::WALLETS_ADD,
                        icon: Icon::Wallet,
                        label: String::from(s.wallets_add),
                    },
                    Row::Action {
                        id: ids::WALLETS_LOAD,
                        icon: Icon::Scan,
                        label: String::from(s.wallet_load),
                    },
                ];
            }
            screens::menu(
                c,
                s.home_wallets,
                None,
                rows,
                vec![Action::new(ids::WALLETS_ADD, s.wallets_add)],
            )
        })
    }

    /// §5 Menu, a wallet's "Keys": the same list filtered to the
    /// wallet's members, each with the wallet glyph where this device
    /// holds it and the eye where it does not. A member that is not loaded
    /// carries §4.11's dimmed reason and opens Add a key.
    pub(crate) fn view_wallet_keys(&self, wallet: usize) -> Node {
        let s = self.strings();
        let members = self.wallet_keys(wallet);
        self.with_chrome(Some(ids::BACK), |c| {
            let rows = self.key_review_rows(ids::WALLET_KEY_ROW_BASE, &members);
            screens::menu(c, s.wallet_keys, None, rows, Vec::new())
        })
    }

    /// §5 Menu, the transaction's "Keys": the same review over the keys
    /// a transaction names rather than a wallet's members
    /// (`docs/PLANNING.md` §16.110 rule 3).
    pub(crate) fn view_sign_keys(&self, reading: bool) -> Node {
        let s = self.strings();
        let members: Vec<(String, Option<usize>)> = self
            .transaction_keys(reading)
            .into_iter()
            .map(|(_, label, loaded)| (label, loaded))
            .collect();
        self.with_chrome(Some(ids::BACK), |c| {
            let rows = self.key_review_rows(ids::SIGN_KEY_ROW_BASE, &members);
            screens::menu(c, s.wallet_keys, None, rows, Vec::new())
        })
    }

    /// One row per member: the wallet glyph where this device holds it,
    /// the eye with §4.11's reason where it does not.
    fn key_review_rows(&self, base: u32, members: &[(String, Option<usize>)]) -> Vec<Row> {
        let s = self.strings();
        members
            .iter()
            .enumerate()
            .map(|(i, (label, loaded))| Row::Menu {
                id: ids::at(base, i),
                icon: Some(self.wallet_glyph(loaded.is_some())),
                label: label.clone(),
                value: loaded.is_none().then(|| String::from(s.key_not_loaded)),
                tone: Tone::Muted,
            })
            .collect()
    }

    /// §5 Menu, "Which key?": the loaded keys, in the shape the wallet
    /// builder's own question has. Asked only while a wallet's or a
    /// transaction's row is waiting for one key in particular
    /// (§16.104 rule 6); a key's own page opens a key from it with no
    /// question (§16.127 rule 3).
    pub(crate) fn view_pick_key(&self, purpose: PickFor) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let rows: Vec<Row> = self
                .keys
                .iter()
                .enumerate()
                .filter(|(_, k)| purpose != PickFor::Passphrase || k.has_mnemonic())
                .map(|(i, _)| Row::Key {
                    id: ids::at(ids::PICK_KEY_BASE, i),
                    fingerprint: text::fingerprint_hex(self.keys[i].fingerprint),
                    subtitle: self.key_made_of(i),
                })
                .collect();
            screens::menu(c, s.build_which_title, None, rows, Vec::new())
        })
    }
}
