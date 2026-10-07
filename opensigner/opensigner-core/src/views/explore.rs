//! Explore's screens (`docs/DESIGN.md` §5): two Menus and one reusable
//! screen behind each of their rows.
//!
//! **Explore** is the key menu — the key it acts on, the path, the
//! fingerprint, the three public encodings as reference rows, the
//! extended private key as a secret row, the addresses and "Words and
//! bits" — and **Words and bits** is the second menu, whose rows open
//! the Words screen and one Secret screen per long secret. The key
//! chooser is a Choice, the path and the passphrase are Entry screens,
//! and the confirm that guards typed words is a Result.
//!
//! Nothing on a working screen here explains anything: the path, the
//! checksum, the seed and the encodings are explained in Learn (§2.1,
//! §4.12). Every secret is masked unless a
//! finger is on its panel or the app bar's eye is running (§4.10);
//! public keys, fingerprints and addresses come from the non-secret
//! cache in `lib.rs`.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::keys::ScriptType;
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{
    self, Above, Action, Chrome, Entry, Item, KeyContext, Result, Row, Secret as SecretScreen,
    Value,
};
use osk_ui::widgets::keyboard::{ALL_KEYS, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::explore::{SecretKind, Source, Step};
use crate::load::EntryList;
use crate::views::words;
use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    pub(crate) fn view_explore(&self) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            if self.explore.source() == Source::None {
                return self.explore_empty(c);
            }
            match self.explore.step() {
                Step::Keys => self.explore_keys(c),
                Step::Chooser => self.explore_chooser(c),
                Step::Path => self.path_editor(c),
                Step::Passphrase => self.passphrase_editor(c),
                Step::Bits => self.explore_bits(c),
                Step::Words => self.explore_words(c),
                Step::Secret(secret) => self.explore_secret(c, secret),
                Step::Discard => self.discard_confirm(c),
            }
        })
    }

    /// §5 Menu, "Explore", with no key and no words: §4.14's empty state
    /// is "a row that starts the flow", not a sentence about emptiness.
    fn explore_empty(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        screens::menu(
            c,
            s.explore_title,
            None,
            vec![
                Row::Menu {
                    id: ids::EXPLORE_TYPE,
                    icon: Some(Icon::List),
                    label: String::from(s.explore_type_words),
                    value: None,
                    tone: Tone::Text,
                },
                Row::Menu {
                    id: ids::EXPLORE_LOAD,
                    icon: Some(Icon::Download),
                    label: String::from(s.home_load_key),
                    value: None,
                    tone: Tone::Text,
                },
            ],
            Vec::new(),
        )
    }

    /// §5 Menu, "Explore": the key context row, the path, the
    /// fingerprint, what the key encodes to, and the rows out. §4.5
    /// makes every long string a reference row and §4.10 every long
    /// secret a secret row, so nothing on this screen is a key drawn in
    /// full: each is one tap away on its own screen.
    fn explore_keys(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let e = &self.explore;
        let cached = self.explore_cached();
        let mut rows = vec![
            Row::Path {
                id: ids::EXPLORE_PATH,
                label: String::from(s.explore_path),
                value: text::path(e.applied_path()),
            },
            Row::Fingerprint {
                id: None,
                label: String::from(s.explore_fingerprint),
                value: self
                    .explore_fingerprint()
                    .map_or_else(|| String::from(s.value_none), text::fingerprint_hex),
            },
        ];
        // §4.5, §2.11: the account key is a row of its own only where it
        // is a key of its own. At a path no deeper than the account it is
        // the leaf, and the "Extended public key" row below already shows
        // that string.
        if cached.is_some_and(|c| c.has_account_key()) {
            rows.push(Row::Reference {
                id: ids::EXPLORE_ACCOUNT_XPUB,
                label: String::from(s.explore_account_xpub),
                value: cached.map(|c| c.account_xpub.clone()).unwrap_or_default(),
            });
        }
        // §4.11: a row that cannot be opened says why in two or three
        // words, rather than leaving the screen to explain it.
        rows.push(match cached.and_then(|c| c.account_slip132.as_ref()) {
            Some((_, value)) => Row::Reference {
                id: ids::EXPLORE_SLIP132,
                label: String::from(s.explore_slip132),
                value: value.clone(),
            },
            // §4.11: the reason names the script types that have a
            // SLIP-132 form, in §4.6's one vocabulary.
            // §3: SLIP-132 is a value row, and a value row carries no
            // glyph whether or not it can be opened.
            None => Row::Dimmed {
                icon: None,
                label: String::from(s.explore_slip132),
                reason: Some(String::from(s.explore_no_slip132)),
            },
        });
        rows.push(Row::Reference {
            id: ids::EXPLORE_XPUB,
            label: String::from(s.explore_level_xpub),
            value: self.explore_xpub(),
        });
        rows.push(Row::Secret {
            id: ids::EXPLORE_XPRV,
            label: String::from(s.explore_level_xprv),
            value: None,
        });
        if e.source() == Source::Typed {
            rows.push(Row::Menu {
                id: ids::EXPLORE_PASSPHRASE,
                icon: None,
                label: String::from(s.detail_passphrase),
                value: Some(String::from(if e.passphrase_len() > 0 {
                    s.explore_passphrase_set
                } else {
                    s.value_none
                })),
                tone: Tone::Text,
            });
        }
        // Typed words are a key, so the row opens the same Addresses
        // screen a loaded key's does; with no words and no key there is
        // nothing to derive from.
        rows.push(match e.source() {
            Source::None => Row::Dimmed {
                icon: Some(Icon::Qr),
                label: String::from(s.explore_addresses),
                reason: Some(String::from(s.reason_needs_key)),
            },
            _ => Row::Menu {
                id: ids::EXPLORE_ADDRESSES,
                icon: Some(Icon::Qr),
                label: String::from(s.explore_addresses),
                value: None,
                tone: Tone::Text,
            },
        });
        rows.push(Row::Menu {
            id: ids::EXPLORE_BITS,
            icon: Some(Icon::Superscript),
            label: String::from(s.explore_section_words),
            value: None,
            tone: Tone::Text,
        });
        screens::menu(
            c,
            s.explore_title,
            Some(self.explore_key_context()),
            rows,
            Vec::new(),
        )
    }

    /// §4.4 Key context: "'Key' above, the fingerprint(s) below, chevron.
    /// Tap opens the key chooser." Typed words have no key row of their
    /// own anywhere else, so the row names them instead.
    fn explore_key_context(&self) -> KeyContext {
        let s = self.strings();
        let name = match (self.explore.source(), self.explore_fingerprint()) {
            (Source::Typed, _) => String::from(s.explore_typed),
            (_, Some(fp)) => text::fingerprint_hex(fp),
            (_, None) => String::from(s.value_none),
        };
        KeyContext {
            id: Some(ids::EXPLORE_USING),
            label: String::from(s.sign_key_row),
            keys: vec![name],
        }
    }

    /// §5 Choice, "Which key?": one key row per loaded key, "Typed
    /// words" when some are typed, the current one checked, Continue
    /// (§2.7).
    fn explore_chooser(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let picked = self.explore.picked();
        let mut items: Vec<Item> = self
            .keys
            .iter()
            .enumerate()
            .map(|(i, k)| {
                Item::key(
                    ids::at(ids::EXPLORE_KEY_BASE, i),
                    text::fingerprint_hex(k.fingerprint),
                    k.network.name(),
                    picked == Source::Loaded(i),
                )
            })
            .collect();
        if self.explore.has_input() {
            items.push(Item::chosen(
                ids::EXPLORE_TYPED,
                s.explore_typed,
                picked == Source::Typed,
            ));
        }
        screens::choice(
            c,
            s.explore_using_title,
            items,
            Action::when(
                ids::EXPLORE_CHOOSE_CONTINUE,
                s.action_continue,
                picked != Source::None,
            ),
        )
    }

    /// §5 Entry, "Path": the field, its reserved error line, the presets
    /// as two labelled groups above them, and the path keyboard. §4.6: "✓
    /// dead while the text does not parse"; ✓ applies the path and the key
    /// menu comes back with it.
    ///
    /// §4.6: "the presets as two labelled groups, each with its own check,
    /// because they are two independent choices. 'Purpose': four choice
    /// rows named in §4.6's vocabulary (Legacy, Nested, SegWit, Taproot)
    /// with the path as the value under the name; 'Chain': the Receive |
    /// Change pair. A row's check follows the typed path, so editing the
    /// text unchecks what no longer matches." The row's name is the script
    /// type and its value is the path that produces it, at this network's
    /// coin type; "BIP-84" appears nowhere.
    fn path_editor(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let e = &self.explore;
        let network = self.network;
        let purpose: Vec<Item> = ScriptType::ALL
            .iter()
            .enumerate()
            .map(|(i, sc)| {
                Item::valued(
                    ids::at(ids::EXPLORE_PRESET_BASE, i),
                    text::script_short(*sc, s),
                    text::preset_path(*sc, network),
                    e.at_preset(*sc, network),
                )
            })
            .collect();
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: s.explore_path,
                value: alloc::format!("m/{}", e.path_text()),
                mono: true,
                above: Above::Presets {
                    purpose: (String::from(s.explore_purpose_group), purpose),
                    chain: (
                        String::from(s.explore_chain_group),
                        (ids::EXPLORE_RECEIVE, String::from(s.addresses_receive)),
                        (ids::EXPLORE_CHANGE, String::from(s.addresses_change)),
                        e.on_change_chain(),
                    ),
                },
                words: None,
                eye: None,
                keyboard: (ids::EXPLORE_PATH_KEYBOARD, KeyboardKind::Path),
                enabled: if e.path_error().is_some() {
                    DONE_DISABLED
                } else {
                    0
                },
                error: e.path_error().map(|err| String::from(err.message(s))),
            },
        )
    }

    /// §5 Entry, "Passphrase": one masked line over its keyboard, the
    /// last character typed still showing for half a second
    /// (`docs/PLANNING.md` §4.6).
    fn passphrase_editor(&self, c: &Chrome<'_>) -> Node {
        let e = &self.explore;
        let n = e.passphrase_len();
        let mut shown: String = core::iter::repeat_n('\u{2022}', n.saturating_sub(1)).collect();
        match e.passphrase_visible_char(self.now_ms) {
            Some(ch) => shown.push(ch),
            None if n > 0 => shown.push('\u{2022}'),
            None => {}
        }
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: self.strings().passphrase_title,
                value: shown,
                mono: true,
                above: Above::Nothing,
                words: None,
                eye: None,
                keyboard: (ids::EXPLORE_PASS_KEYBOARD, KeyboardKind::Passphrase),
                enabled: ALL_KEYS,
                error: None,
            },
        )
    }

    /// §5 Menu, "Words and bits": the words, the entropy they encode,
    /// the checksum over it, the seed the words stretch into and the key
    /// that comes out. Each is a screen of its own (§2.8); the row
    /// states only how many checksum bits there are, because that is a
    /// fact and not a secret.
    fn explore_bits(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        // A SLIP-39 key has no words, so the four rows over them are
        // not drawn: the seed it is and the key that comes out of it
        // are (§16.107).
        let bits = self.explore_with_mnemonic(|m| m.checksum_bits().1);
        let mut rows = Vec::new();
        if let Some(bits) = bits {
            rows.push(Row::Menu {
                id: ids::EXPLORE_WORDS,
                icon: Some(Icon::List),
                label: String::from(s.load_words_row),
                value: None,
                tone: Tone::Text,
            });
            rows.push(Row::Secret {
                id: ids::EXPLORE_ENTROPY,
                label: String::from(s.explore_entropy),
                value: None,
            });
            rows.push(Row::Fact {
                label: String::from(s.create_math_checksum),
                value: strings::fill1(s.create_math_bits, &alloc::format!("{bits}")),
                mono: false,
                tone: Tone::Text,
            });
            rows.push(Row::Secret {
                id: ids::EXPLORE_CHECKSUM_BITS,
                label: String::from(s.explore_checksum),
                value: None,
            });
        }
        rows.push(Row::Secret {
            id: ids::EXPLORE_SEED,
            label: String::from(s.explore_seed),
            value: None,
        });
        rows.push(Row::Secret {
            id: ids::EXPLORE_MASTER_XPRV,
            label: String::from(s.explore_master_xprv),
            value: None,
        });
        screens::menu(c, s.explore_section_words, None, rows, Vec::new())
    }

    /// §5 Words: the one words screen every flow uses, over whatever
    /// Explore derives from.
    fn explore_words(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let e = &self.explore;
        let Some(node) = self.explore_with_mnemonic(|m| {
            words::words_screen(
                c,
                words::WordsScreen {
                    title: String::from(s.words_title),
                    list: EntryList::Bip39(m.language()),
                    indices: m.indices(),
                    revealed: self.revealed(ids::CREATE_REVEAL),
                    rows: words::Rows::Words {
                        numbers: e.numbers(),
                    },
                    page: e.word_page(),
                    remaining: self.reveal_ring(),
                    extra: None,
                    action: Action::new(ids::EXPLORE_WORDS_DONE, s.action_done),
                },
                s,
            )
        }) else {
            return screens::menu(c, s.load_words_row, None, Vec::new(), Vec::new());
        };
        node
    }

    /// §5 Secret: one long secret, whole, on its own screen, with the
    /// panel that masks it and the app bar's eye (§2.8, §4.10). The
    /// title says what is on the screen, so nothing under the panel has
    /// to.
    fn explore_secret(&self, c: &Chrome<'_>, secret: SecretKind) -> Node {
        let s = self.strings();
        let (title, value) = match secret {
            SecretKind::Xprv => (s.explore_level_xprv, self.explore_xprv()),
            SecretKind::Entropy => (
                s.explore_entropy,
                self.explore_with_mnemonic(|m| text::hex(m.entropy().expose().as_bytes()))
                    .unwrap_or_default(),
            ),
            SecretKind::ChecksumBits => (
                s.explore_checksum,
                self.explore_with_mnemonic(|m| {
                    let (checksum, bits) = m.checksum_bits();
                    binary(u16::from(checksum), usize::from(bits))
                })
                .unwrap_or_default(),
            ),
            SecretKind::Seed => (
                s.explore_seed,
                self.explore_with_seed(text::hex).unwrap_or_default(),
            ),
            SecretKind::MasterXprv => (
                s.explore_master_xprv,
                self.explore_master()
                    .map(|m| String::from(m.xpriv_ascii().as_str()))
                    .unwrap_or_default(),
            ),
        };
        screens::secret(
            c,
            SecretScreen {
                rows: Vec::new(),
                action: None,
                title,
                value: Value::Text(&value),
                revealed: self.revealed(ids::CREATE_REVEAL),
                panel: ids::CREATE_REVEAL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
                secondary: None,
                pager: None,
            },
        )
    }

    /// §5 Result: what leaving with typed words costs, as one fact, and
    /// the two ways out. §4.13 gives the accent to the safe one.
    fn discard_confirm(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let words = self
            .explore_with_mnemonic(osk_bip::bip39::Mnemonic::word_count)
            .unwrap_or(0);
        screens::result(
            c,
            Result {
                caption: None,
                title: s.explore_title,
                icon: Icon::Warning,
                tone: Tone::Caution,
                result: s.explore_discard_title,
                rows: vec![components::Record::text(
                    s.load_words_row,
                    alloc::format!("{words}"),
                    Tone::Text,
                )],
                actions: vec![
                    Action::new(ids::EXPLORE_DISCARD, s.explore_discard),
                    Action::new(ids::EXPLORE_KEEP, s.explore_keep),
                ],
            },
        )
    }

    /// The extended public key at the applied path, from the non-secret
    /// cache: the master's when the path is `m`, the leaf's otherwise.
    fn explore_xpub(&self) -> String {
        let Some(cached) = self.explore_cached() else {
            return String::new();
        };
        match cached.levels.last() {
            Some(level) => level.xpub.clone(),
            None => cached.master_xpub.clone(),
        }
    }

    /// The extended private key at the applied path, derived for the one
    /// screen that shows it and never cached.
    fn explore_xprv(&self) -> String {
        let path = self.explore.applied_path().clone();
        self.explore_master()
            .map(|m| String::from(m.derive(&path).xpriv_ascii().as_str()))
            .unwrap_or_default()
    }
}

/// `v` as `n` binary digits, most significant first.
fn binary(v: u16, n: usize) -> String {
    (0..n)
        .rev()
        .map(|b| if (v >> b) & 1 == 1 { '1' } else { '0' })
        .collect()
}
