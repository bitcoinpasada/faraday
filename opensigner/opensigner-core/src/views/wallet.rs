//! The page of one wallet (`docs/DESIGN.md` §5 Menu): the actions, then
//! the keys it is made of, then Forget.
//!
//! Every kind of wallet has the same shape. What differs is which
//! actions exist: a wallet this device holds no private key of has no
//! Sign rows, and only a wallet over one key can sign a message, since a
//! message is signed by one key. A wallet is always a registered policy
//! (`docs/PLANNING.md` §16.104 rule 2); a key has no page of this kind.
//!
//! The title is what a person calls the wallet — its name, or
//! "73c5da0a", "2 of 3 · SegWit" — so nothing in the body repeats it.

use alloc::string::String;
use alloc::vec::Vec;

use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Entry, Row};
use osk_ui::widgets::keyboard::{self, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::{OpenSigner, WalletRef, ids};

impl OpenSigner {
    /// One row of a wallet's page.
    fn wallet_row(id: ids::Id, icon: Icon, label: &str) -> Row {
        Row::Menu {
            id,
            icon: Some(icon),
            label: String::from(label),
            value: None,
            tone: Tone::Text,
        }
    }

    /// The rows of `wallet`: what can be done with it, the key or keys
    /// behind it, and the way to drop it.
    fn wallet_rows(&self, wallet: WalletRef) -> Vec<Row> {
        let s = self.strings();
        let mut rows = Vec::new();
        // §4.2 Value: the name, which is the one thing about a wallet a
        // person sets. It is the first row because it is what tells two
        // wallets of one shape apart. A wallet with no name has nothing
        // to state, so the way to give it one is a row under the
        // actions instead.
        if let Some(name) = self.wallet_name(wallet) {
            rows.push(Row::Value {
                id: ids::WALLET_NAME,
                label: String::from(s.wallet_name),
                value: String::from(name),
                tone: Tone::Text,
            });
        }
        // §16.113: a silent payments wallet has its own rows. Its
        // coins are taproot outputs a sender computed, so there is no
        // chain of addresses to list and no address of another wallet's
        // to check against it; and sending to a silent payment address
        // is not built, so there is no Sign row.
        if self.is_silent(wallet) {
            rows.push(Self::wallet_row(
                ids::SILENT_ADDRESS,
                Icon::Qr,
                s.silent_address_row,
            ));
            rows.push(Self::wallet_row(
                ids::SILENT_LABELS,
                Icon::Numbers,
                s.silent_labels_row,
            ));
            rows.push(Self::wallet_row(
                ids::SILENT_CHECK,
                Icon::Verify,
                s.silent_check_row,
            ));
            rows.push(Self::wallet_row(
                ids::WALLET_EXPORT,
                Icon::Export,
                s.wallet_export,
            ));
            if self.wallet_name(wallet).is_none() {
                rows.push(Self::wallet_row(
                    ids::WALLET_NAME,
                    Icon::Keyboard,
                    s.wallet_set_name,
                ));
            }
            rows.push(Self::wallet_row(
                ids::WALLET_KEYS,
                Icon::Fingerprint,
                s.wallet_keys,
            ));
            if let WalletRef::Policy(policy) = wallet
                && self.wallet_keep_offered()
            {
                rows.push(Row::Toggle {
                    id: ids::WALLET_KEEP,
                    label: String::from(s.wallet_keep_row),
                    on: self.wallet_is_kept(policy),
                    reason: None,
                });
            }
            rows.push(Self::wallet_row(
                ids::WALLET_FORGET,
                Icon::Trash,
                s.wallet_forget,
            ));
            return rows;
        }
        if self.wallet_can_sign(wallet) {
            rows.push(Self::wallet_row(
                ids::WALLET_SIGN,
                Icon::Sign,
                s.wallet_sign,
            ));
            if self.wallet_signs_messages(wallet) {
                rows.push(Self::wallet_row(
                    ids::WALLET_SIGN_MESSAGE,
                    Icon::Envelope,
                    s.wallet_sign_message,
                ));
            }
        }
        rows.push(Self::wallet_row(
            ids::WALLET_ADDRESSES,
            Icon::Qr,
            s.detail_addresses,
        ));
        rows.push(Self::wallet_row(
            ids::WALLET_CHECK,
            Icon::Verify,
            s.wallet_check,
        ));
        rows.push(Self::wallet_row(
            ids::WALLET_EXPORT,
            Icon::Export,
            s.wallet_export,
        ));
        // §16.112 rule 2: the descriptor, the name and a note, which is
        // the document an heir is left with. Only a registered policy
        // has one, because only a policy has a descriptor to write down.
        if matches!(wallet, WalletRef::Policy(_)) {
            rows.push(Self::wallet_row(
                ids::WALLET_SHEET,
                Icon::Receipt,
                s.wallet_sheet_row,
            ));
        }
        if self.wallet_name(wallet).is_none() {
            rows.push(Self::wallet_row(
                ids::WALLET_NAME,
                Icon::Keyboard,
                s.wallet_set_name,
            ));
        }
        // The Keys row opens the Keys screen filtered to the wallet's
        // members, for every kind (§16.104 rules 2 and 3).
        rows.push(Self::wallet_row(
            ids::WALLET_KEYS,
            Icon::Fingerprint,
            s.wallet_keys,
        ));
        // §4.2 Toggle: whether this wallet is in the blob, on a Tier B
        // device that keeps keys and nowhere else (`docs/PLANNING.md`
        // §16.104 rule 7).
        if let WalletRef::Policy(policy) = wallet
            && self.wallet_keep_offered()
        {
            rows.push(Row::Toggle {
                id: ids::WALLET_KEEP,
                label: String::from(s.wallet_keep_row),
                on: self.wallet_is_kept(policy),
                reason: None,
            });
        }
        rows.push(Self::wallet_row(
            ids::WALLET_FORGET,
            Icon::Trash,
            s.wallet_forget,
        ));
        rows
    }

    pub(crate) fn view_wallet(&self, wallet: WalletRef) -> Node {
        let s = self.strings();
        let present = match wallet {
            WalletRef::Key(key) => self.keys.get(key).is_some(),
            WalletRef::Policy(policy) => self.wallets.get(policy).is_some(),
            WalletRef::Typed => false,
        };
        let (title, rows) = match present {
            false => (String::from(s.wallet_title), Vec::new()),
            true => (self.wallet_call_name(wallet), self.wallet_rows(wallet)),
        };
        let mono = present && self.wallet_called_by_key(wallet);
        self.with_chrome(Some(ids::BACK), |c| {
            if mono {
                screens::menu_mono_title(c, &title, None, rows, Vec::new())
            } else {
                screens::menu(c, &title, None, rows, Vec::new())
            }
        })
    }

    /// §5 Entry, "Name": the wallet's name, typed on the passphrase
    /// keyboard. ✓ is live whatever is in the field, because an empty
    /// name takes the name away and leaves the shape.
    pub(crate) fn view_wallet_name(&self, _wallet: WalletRef) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            screens::entry(
                c,
                Entry {
                    candidates: None,
                    title: s.wallet_name,
                    value: self.name_entry.clone(),
                    mono: false,
                    above: Above::Nothing,
                    words: None,
                    eye: None,
                    keyboard: (ids::WALLET_NAME_KEYBOARD, KeyboardKind::Passphrase),
                    enabled: keyboard::ALL_KEYS,
                    error: None,
                },
            )
        })
    }
}
