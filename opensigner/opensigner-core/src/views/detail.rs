//! Key detail and its screens, built from `docs/DESIGN.md` §5.
//!
//! A key's page is a Menu over one secret: what the key is made of,
//! what is done with this key — its account key, BIP-85, "Open with
//! passphrase", a vanity grind — then Backup, Keep on this device,
//! Forget. Nothing about a wallet is here — no addresses, no export, no
//! signing — because a key is not a wallet (`docs/PLANNING.md` §16.104
//! rule 1). A key opens another key from here and not from Add a key,
//! which is the ways a key is made (§16.127 rule 3).

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::keys::{Network, ScriptType};
use osk_bip::policy::WalletPolicy;
use osk_codec::qr::{Ecc, Payload};
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, AddressRow, Chrome, Entry, Hold, Item, Row};
use osk_ui::widgets::keyboard::{ALL_KEYS, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::finish::Finish;
use crate::load::KeyKind;
use crate::{CHILD_COUNTS, ChildStep, OpenSigner, WalletRef, ids, strings, text};

impl OpenSigner {
    /// The key's title, which is its fingerprint.
    fn key_title(&self, key: usize) -> String {
        match self.keys.get(key) {
            Some(k) => text::fingerprint_hex(k.fingerprint),
            None => String::from(self.strings().detail_title),
        }
    }

    /// What the key is made of, which is the one fact its page states:
    /// the words it holds, the words and a passphrase, a BIP-85 child,
    /// or the SLIP-39 shares it was read from. A backup needs it;
    /// nothing else does.
    pub(crate) fn key_made_of(&self, key: usize) -> String {
        let s = self.strings();
        let k = match self.keys.get(key) {
            Some(k) => k,
            None => return String::from(s.value_none),
        };
        let words = self
            .key_word_count(key)
            .map(|n| alloc::format!("{n}"))
            .unwrap_or_default();
        if !k.has_mnemonic() {
            // A key with no words is its seed: a SLIP-39 master secret,
            // whose shares are the only form it is written in, or a
            // codex32 string, which BIP 93 gives no passphrase.
            if k.kind() == KeyKind::Codex32 {
                return String::from(s.key_made_codex32);
            }
            return String::from(if k.has_passphrase {
                s.key_made_slip39_passphrase
            } else {
                s.key_made_slip39
            });
        }
        match (k.has_passphrase, k.derived) {
            (true, _) => strings::fill1(s.key_made_passphrase, &words),
            (false, true) => String::from(s.key_made_child),
            (false, false) => strings::fill1(s.key_made_words, &words),
        }
    }

    /// §4.14 Empty state: a screen for a key that is no longer loaded is
    /// the same screen with nothing in it. There is no sentence about
    /// emptiness, and the chevron is the way out.
    fn missing_key(&self, c: &Chrome<'_>, title: &str) -> Node {
        screens::menu(c, title, None, Vec::new(), Vec::new())
    }

    /// §5 Menu, titled with the key's fingerprint: what the key is made
    /// of, Backup, Keep on this device, Forget. The title is the
    /// identity, so nothing in the body repeats the fingerprint; a row
    /// that states a value carries it under its label (§4.1).
    pub(crate) fn view_detail(&self, key: usize) -> Node {
        let s = self.strings();
        let title = self.key_title(key);
        let rows = match self.keys.get(key) {
            None => Vec::new(),
            Some(k) => {
                let mut rows: Vec<Row> = vec![Row::Flat {
                    label: String::from(s.key_made_of),
                    value: self.key_made_of(key),
                    mono: false,
                }];
                // §16.110 rule 1: a key exports its public accounts,
                // and which account is the one question the row asks.
                if self.key_has_master(key) {
                    rows.push(Row::Menu {
                        id: ids::DETAIL_ACCOUNT,
                        icon: Some(Icon::Qr),
                        label: String::from(s.key_account_row),
                        value: None,
                        tone: Tone::Text,
                    });
                }
                // §16.114: every key with a master derives BIP-85's
                // applications from it, whatever the key is made of.
                if self.key_has_master(key) {
                    rows.push(Row::Menu {
                        id: ids::DETAIL_BIP85,
                        icon: Some(Icon::Derivation),
                        label: String::from(s.key_bip85_row),
                        value: None,
                        tone: Tone::Text,
                    });
                }
                // §16.127 rule 3: opening a key with a passphrase is
                // done with this key, so its row is this key's and
                // needs no picker. Only a key whose words this device
                // holds has a passphrase to append.
                if k.has_mnemonic() {
                    rows.push(Row::Menu {
                        id: ids::DETAIL_OPEN_PASSPHRASE,
                        icon: Some(Icon::Passphrase),
                        label: String::from(s.detail_open_passphrase),
                        value: None,
                        tone: Tone::Text,
                    });
                }
                // §16.129 rule 1: a wallet over this key starts here,
                // with the key in hand.
                if self.key_has_master(key) {
                    rows.push(Row::Menu {
                        id: ids::DETAIL_ADD_WALLET,
                        icon: Some(Icon::Wallet),
                        label: String::from(s.wallets_add),
                        value: None,
                        tone: Tone::Text,
                    });
                }
                // §16.117: the grinder turns one of the key's own
                // dials, so its row is the key's.
                if self.key_has_master(key) {
                    rows.push(Row::Menu {
                        id: ids::DETAIL_VANITY,
                        icon: Some(Icon::Bitcoin),
                        label: String::from(s.key_vanity_row),
                        value: None,
                        tone: Tone::Text,
                    });
                }
                // §4.8: an unverified backup is a state to act on, so
                // it is a row value in the caution tone rather than a
                // badge.
                // §5: a row whose value is a neutral state keeps the
                // value in the body tone; only the label is muted, so
                // the row does not read as dead beside the plain ones.
                // A key with no words has no quiz to pass, so the row
                // states no backup state at all.
                let (state, tone) = if k.backup_verified {
                    (s.detail_backup_verified, Tone::Text)
                } else {
                    (s.detail_backup_unverified, Tone::Caution)
                };
                rows.push(Row::Menu {
                    id: ids::DETAIL_BACKUP,
                    icon: Some(Icon::Shield),
                    label: String::from(s.detail_backup),
                    value: k.has_mnemonic().then(|| String::from(state)),
                    tone: if k.has_mnemonic() { tone } else { Tone::Text },
                });
                // §5 Menu, key menu: "Keep on this device" on a Tier B
                // device with secure hardware, and nowhere else. Once
                // the key is kept the row states it, and its value is
                // what is holding the storage key.
                if self.keep_is_kept(key) {
                    rows.push(Row::Value {
                        id: ids::KEEP_ROW,
                        label: String::from(s.keep_row_kept),
                        value: String::from(self.secure_name()),
                        tone: Tone::Text,
                    });
                } else if self.keep_offered(key) {
                    rows.push(Row::Menu {
                        id: ids::KEEP_ROW,
                        icon: Some(Icon::Drive),
                        label: String::from(s.keep_row),
                        value: None,
                        tone: Tone::Text,
                    });
                }
                // Forgetting a key cannot be undone, so the row opens
                // the hold (§2.7).
                rows.push(Row::Menu {
                    id: ids::KEY_FORGET,
                    icon: Some(Icon::Trash),
                    label: String::from(s.wallet_forget),
                    value: None,
                    tone: Tone::Text,
                });
                rows
            }
        };
        self.with_chrome(Some(ids::BACK), |c| {
            screens::menu_mono_title(c, &title, None, rows, Vec::new())
        })
    }

    /// §5 Addresses and §5 Address: the list of addresses a key or a
    /// wallet derives, and one of them on a screen of its own.
    pub(crate) fn view_addresses(&self, owner: WalletRef) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            let d = &self.detail;
            let addresses = match owner {
                WalletRef::Key(key) => {
                    let Some(k) = self.keys.get(key) else {
                        return self.missing_key(c, s.addresses_title);
                    };
                    self.cached_addresses(k.fingerprint, d.script, d.change)
                }
                WalletRef::Policy(wallet) => {
                    if self.wallets.get(wallet).is_none() {
                        return self.missing_key(c, s.addresses_title);
                    }
                    self.wallet_addresses(wallet, d.change)
                }
                // Explore's typed words, which derive their own
                // addresses at the script type this screen has chosen.
                WalletRef::Typed => match self.typed_fingerprint() {
                    Some(fp) => self.cached_addresses(fp, d.script, d.change),
                    None => return self.missing_key(c, s.addresses_title),
                },
            };
            match d.open {
                Some(i) => self.view_address(c, addresses.get(i as usize), i, owner),
                None => self.view_address_list(c, addresses, owner),
            }
        })
    }

    /// §5 Addresses: "script type value row, Receive | Change pair, then
    /// one reference row per address." The run has no end, so the list
    /// grows from its own last row rather than paging (§4.1).
    fn view_address_list(&self, c: &Chrome<'_>, addresses: &[String], owner: WalletRef) -> Node {
        let s = self.strings();
        let d = &self.detail;
        let rows: Vec<AddressRow> = addresses
            .iter()
            .take(d.shown)
            .enumerate()
            .map(|(i, address)| AddressRow {
                id: ids::at(ids::ADDR_ROW_BASE, i),
                label: self.address_row_label(i as u32),
                address: address.clone(),
            })
            .collect();
        let more = (d.shown <= crate::MAX_ADDRESS_INDEX as usize)
            .then(|| (ids::ADDR_MORE, String::from(s.action_more)));
        screens::addresses(
            c,
            screens::Addresses {
                title: s.addresses_title,
                // A key's addresses are offered at any of the four
                // script types; a wallet pays to the one its template
                // names, which is a fact and opens nothing (§4.12).
                script: (
                    matches!(owner, WalletRef::Key(_)).then_some(ids::ADDR_SCRIPT),
                    s.script_type_row,
                    text::script_short(self.address_script(owner), s),
                ),
                chain: (
                    ids::ADDR_RECEIVE,
                    s.addresses_receive,
                    ids::ADDR_CHANGE,
                    s.addresses_change,
                    d.change,
                ),
                rows,
                more,
            },
        )
    }

    /// The script type the listed addresses pay to: the one the key's
    /// screen has chosen, or the one the wallet's template names.
    fn address_script(&self, owner: WalletRef) -> ScriptType {
        match owner {
            WalletRef::Key(_) | WalletRef::Typed => self.detail.script,
            WalletRef::Policy(wallet) => self
                .wallets
                .get(wallet)
                .map_or(self.detail.script, WalletPolicy::script_type),
        }
    }

    /// `Receive 2`: what one row of the list, and the screen it opens,
    /// are called.
    fn address_row_label(&self, index: u32) -> String {
        let s = self.strings();
        strings::fill(
            s.addresses_row,
            &[
                if self.detail.change {
                    s.addresses_change
                } else {
                    s.addresses_receive
                },
                &alloc::format!("{index}"),
            ],
        )
    }

    /// §5 Address: "the QR at the class's side, the label and the whole
    /// address under it." One address, one screen, and the chevron is the
    /// way back.
    fn view_address(
        &self,
        c: &Chrome<'_>,
        address: Option<&String>,
        index: u32,
        owner: WalletRef,
    ) -> Node {
        let s = self.strings();
        let label = self.address_row_label(index);
        let Some(address) = address else {
            return screens::menu(c, &label, None, Vec::new(), Vec::new());
        };
        let Ok(matrix) = osk_codec::qr::encode(Payload::Bytes(address.as_bytes()), Ecc::Low) else {
            return screens::result(
                c,
                screens::Result {
                    caption: None,
                    title: &label,
                    icon: Icon::Error,
                    tone: Tone::Caution,
                    result: s.export_too_long,
                    rows: Vec::new(),
                    actions: Vec::new(),
                },
            );
        };
        screens::address(
            c,
            screens::Address {
                // §4.5 keeps the whole address on the screen at the
                // largest size that fits, and a 268 dp panel has no room
                // for a row under the code as well: the address is
                // copied from the Compare screen its row opens.
                copy: None,
                format: None,
                // For the same reason the 268 dp panel has no Save as
                // PNG row: the address whole comes first (§16.134).
                save: self.address_png_row(c),
                caption: self.notice().map(String::from),
                title: &label,
                // The title names the address; the label under the code
                // says what kind it is rather than the name again.
                label: text::script_short(self.address_script(owner), s),
                address: (ids::ADDR_TEXT, address),
                code: Rc::new(matrix),
            },
        )
    }

    /// §5 Entry, "Open passphrase": the passphrase over the words this
    /// key already holds, as one masked line over its keyboard. The last
    /// character typed stays visible for half a second, as it does in
    /// the wizard.
    pub(crate) fn view_open_passphrase(&self, key: usize) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            if self.keys.get(key).is_none() {
                return self.missing_key(c, s.detail_open_passphrase);
            }
            let n = self.open_finish.as_ref().map_or(0, Finish::passphrase_len);
            let mut masked: String =
                core::iter::repeat_n('\u{2022}', n.saturating_sub(1)).collect();
            match self
                .open_finish
                .as_ref()
                .and_then(|f| f.passphrase_visible_char(self.now_ms))
            {
                Some(ch) => masked.push(ch),
                None if n > 0 => masked.push('\u{2022}'),
                None => {}
            }
            // §16.129 rule 2: opened from a wallet being built, the
            // entry adds a key the wallet goes on over.
            let title = if self.build_parked.is_some() {
                s.build_passphrase_key_title
            } else {
                s.detail_open_passphrase
            };
            // §4.3's fact row: the key this passphrase opens, as it is
            // typed. Nothing typed is the key that is already loaded, so
            // the row states no key rather than the parent's (§16.67).
            let fingerprint = self
                .open_finish
                .as_ref()
                .and_then(Finish::fingerprint_with_passphrase)
                .map_or_else(|| String::from(s.value_none), text::fingerprint_hex);
            screens::entry(
                c,
                Entry {
                    candidates: None,
                    title,
                    value: masked,
                    mono: true,
                    above: screens::Above::Fact(screens::FactRow::fingerprint(
                        s.confirm_key,
                        fingerprint,
                    )),
                    words: None,
                    eye: None,
                    keyboard: (ids::OPEN_PASS_KEYBOARD, KeyboardKind::Passphrase),
                    // §4.3: ✓ is dead until the input is acceptable, and
                    // an empty passphrase is the key that is already here.
                    enabled: if n == 0 {
                        ALL_KEYS | DONE_DISABLED
                    } else {
                        ALL_KEYS
                    },
                    // The passphrase opened a key that is not the member
                    // the wallet's row named, and nothing was kept
                    // (§16.104 rule 6).
                    error: self.open_error.clone(),
                },
            )
        })
    }

    /// §5 Choice then §5 Entry, "Open BIP-85 child seed": how many words
    /// the child has, then which one it is.
    pub(crate) fn view_open_child(&self, key: usize) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            let Some(child) = self.child.as_ref() else {
                return self.missing_key(c, s.detail_open_child);
            };
            if self.keys.get(key).is_none() {
                return self.missing_key(c, s.detail_open_child);
            }
            match child.step {
                ChildStep::Words => screens::choice(
                    c,
                    s.load_count_title,
                    CHILD_COUNTS
                        .iter()
                        .map(|n| {
                            let i = CHILD_COUNTS
                                .iter()
                                .position(|c| c == n)
                                .expect("a listed count");
                            Item::chosen(
                                ids::at(ids::OPEN_CHILD_WORDS_BASE, i),
                                alloc::format!("{n}"),
                                *n == child.words,
                            )
                        })
                        .collect(),
                    Action::new(ids::OPEN_CHILD_CONTINUE, s.action_continue),
                ),
                ChildStep::Index => screens::entry(
                    c,
                    Entry {
                        candidates: None,
                        title: s.open_child_index,
                        value: String::from(child.index.as_str()),
                        mono: true,
                        // The child the index now names, as it is typed.
                        // An empty or out-of-range index names none.
                        above: screens::Above::Fact(screens::FactRow::fingerprint(
                            s.confirm_key,
                            child
                                .fingerprint
                                .map_or_else(|| String::from(s.value_none), text::fingerprint_hex),
                        )),
                        words: None,
                        eye: None,
                        keyboard: (ids::OPEN_CHILD_KEYBOARD, KeyboardKind::Pin),
                        enabled: if child.index_value().is_some() {
                            ALL_KEYS
                        } else {
                            ALL_KEYS | DONE_DISABLED
                        },
                        error: self.open_error.clone().or_else(|| {
                            child
                                .index_error()
                                .then(|| String::from(s.open_child_index_range))
                        }),
                    },
                ),
            }
        })
    }

    /// §5 Result, after either Open row: which key was added, and the
    /// action that opens its wallet. The chevron goes to the key list,
    /// since the key is added either way (§16.67).
    pub(crate) fn view_opened(&self, key: usize, from: crate::OpenedFrom) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            let title = match from {
                crate::OpenedFrom::Passphrase => s.opened_title,
                crate::OpenedFrom::Child => s.opened_child_title,
            };
            let Some(k) = self.keys.get(key) else {
                return self.missing_key(c, title);
            };
            let mut rows = vec![components::Record::fingerprint(
                s.confirm_key,
                text::fingerprint_hex(k.fingerprint),
            )];
            if let Some(words) = self.key_word_count(key) {
                rows.push(components::Record::text(
                    s.load_words_row,
                    alloc::format!("{words}"),
                    Tone::Text,
                ));
            }
            screens::result(
                c,
                screens::Result {
                    caption: None,
                    title,
                    icon: Icon::Success,
                    tone: Tone::Success,
                    result: s.opened_result,
                    rows,
                    actions: vec![Action::new(ids::OPENED_OPEN, s.opened_open)],
                },
            )
        })
    }

    /// §5 Hold, "Forget key": what will happen as a table, and the hold.
    /// §4.13 puts no sentence above the button.
    pub(crate) fn view_forget(&self, key: usize) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            let Some(k) = self.keys.get(key) else {
                return self.missing_key(c, s.forget_title);
            };
            let mut rows = vec![
                components::Record::fingerprint(
                    s.sign_key_row,
                    text::fingerprint_hex(k.fingerprint),
                ),
                components::Record::text(s.confirm_network, k.network.name(), Tone::Text),
            ];
            // A policy this key is a cosigner of stays, and stops being
            // a wallet this device can sign for: one row per policy,
            // under the name the wallet goes by on Home.
            for p in self.wallets.iter().filter(|p| {
                p.keys()
                    .iter()
                    .any(|pk| pk.fingerprint() == Some(k.fingerprint))
            }) {
                rows.push(components::Record::text(
                    self.policy_call_name(p),
                    s.forget_policy,
                    Tone::Caution,
                ));
            }
            // The key this device keeps goes from the device as well
            // (§16.65), and the table says so before the hold.
            if self.keep_is_kept(key) {
                rows.push(components::Record::text(
                    s.keep_stored_row,
                    s.settings_wipe_stored,
                    Tone::Text,
                ));
            }
            screens::hold(
                c,
                Hold {
                    warnings: Vec::new(),
                    title: s.forget_title,
                    rows,
                    sign_with: None,
                    then_with: None,
                    id: ids::DETAIL_FORGET,
                    label: s.forget_hold,
                    danger: true,
                    enabled: true,
                    secondary: None,
                },
            )
        })
    }
}

/// `m/84h/0h/0h` for `script` on `network`, with the `h` this app writes
/// everywhere else (UX review 2026-09-07, §2.1).
pub(crate) fn account_path(script: ScriptType, network: Network) -> String {
    alloc::format!("m/{}h/{}h/0h", script.purpose(), network.coin_type())
}
