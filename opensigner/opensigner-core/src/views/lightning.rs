//! Tools › Lightning node key (`docs/PLANNING.md` §16.116), built from
//! `docs/DESIGN.md` §5: the Entry that types an LND cipher seed's
//! passphrase, the Result that states the node's public key, and the
//! Secret screen behind "Show secret".

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{
    self, Above, Action, Chrome, Entry, Item, Result, Secret as SecretScreen, Value,
};
use osk_ui::widgets::keyboard::{ALL_KEYS, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::lightning::{Answer, SecretKind, Source, Step};
use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    /// The three screens of the tool.
    pub(crate) fn view_lightning(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let Some(flow) = self.lightning.as_ref() else {
                return screens::menu(c, s.tools_lightning, None, Vec::new(), Vec::new());
            };
            match flow.step() {
                Step::Passphrase => self.lightning_passphrase(c),
                Step::Result => self.lightning_result(c),
                Step::Secret(which) => self.lightning_secret(c, which),
            }
        })
    }

    /// §5 Entry, "Passphrase": one masked line over its keyboard, the
    /// last character typed still showing for half a second (§4.6). An
    /// empty field is LND's own default, so ✓ is live from the start.
    fn lightning_passphrase(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let Some(flow) = self.lightning.as_ref() else {
            return screens::menu(c, s.passphrase_title, None, Vec::new(), Vec::new());
        };
        let n = flow.passphrase_len();
        let mut shown: String = core::iter::repeat_n('\u{2022}', n.saturating_sub(1)).collect();
        match flow.passphrase_visible_char(self.now_ms) {
            Some(ch) => shown.push(ch),
            None if n > 0 => shown.push('\u{2022}'),
            None => {}
        }
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: s.passphrase_title,
                value: shown,
                mono: true,
                above: Above::Nothing,
                words: None,
                eye: None,
                keyboard: (ids::LIGHTNING_PASS_KEYBOARD, KeyboardKind::Passphrase),
                enabled: ALL_KEYS,
                error: None,
            },
        )
    }

    /// §5 Result: the node's public key as a reference row, and what
    /// the source stated beside it. A cipher seed that does not open is
    /// the same screen with the reason on it.
    fn lightning_result(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let Some(flow) = self.lightning.as_ref() else {
            return screens::menu(c, s.tools_lightning, None, Vec::new(), Vec::new());
        };
        match flow.answer() {
            None | Some(Answer::Refused(_)) => {
                let reason = match flow.answer() {
                    Some(Answer::Refused(e)) => refusal(*e, s),
                    _ => s.lightning_no_answer,
                };
                screens::result(
                    c,
                    Result {
                        caption: None,
                        title: s.tools_lightning,
                        icon: Icon::Warning,
                        tone: Tone::Caution,
                        result: reason,
                        rows: Vec::new(),
                        actions: vec![Action::new(ids::LIGHTNING_DONE, s.action_done)],
                    },
                )
            }
            Some(Answer::Node(node)) => {
                let mut rows = vec![components::Record::reference(
                    ids::LIGHTNING_NODE_KEY,
                    String::from(s.lightning_node_key),
                    text::hex(&node.public),
                )];
                match &node.seed {
                    Some(cipher) => {
                        rows.push(components::Record::text(
                            s.lightning_version,
                            alloc::format!("{}", cipher.version),
                            Tone::Text,
                        ));
                        let (y, m, d) = osk_bip::aezeed::birthday_date(cipher.birthday);
                        rows.push(components::Record::text(
                            s.lightning_birthday,
                            strings::fill(
                                s.lightning_birthday_value,
                                &[
                                    &alloc::format!("{y:04}-{m:02}-{d:02}"),
                                    &alloc::format!("{}", cipher.birthday),
                                ],
                            ),
                            Tone::Text,
                        ));
                    }
                    None => {
                        if let Source::Loaded(i) = flow.source()
                            && let Some(key) = self.keys.get(i)
                        {
                            rows.push(components::Record::fingerprint(
                                s.lightning_key_row,
                                text::fingerprint_hex(key.fingerprint),
                            ));
                        }
                    }
                }
                rows.push(components::Record::text(
                    s.lightning_software_row,
                    String::from(match flow.source() {
                        Source::Aezeed => s.lightning_software_lnd,
                        Source::Loaded(_) => s.lightning_software_ldk,
                    }),
                    Tone::Text,
                ));
                screens::result(
                    c,
                    Result {
                        caption: None,
                        title: s.tools_lightning,
                        icon: Icon::Success,
                        tone: Tone::Success,
                        result: match flow.source() {
                            Source::Aezeed => s.lightning_read,
                            Source::Loaded(_) => s.lightning_derived,
                        },
                        rows,
                        actions: vec![
                            Action::new(ids::LIGHTNING_SECRET, s.lightning_show_secret),
                            Action::new(ids::LIGHTNING_DONE, s.action_done),
                        ],
                    },
                )
            }
        }
    }

    /// §5 Secret: the node private key, whole, masked until a finger is
    /// on the panel or the eye is running. Where the source is a cipher
    /// seed the entropy is the second secret, and the action beside
    /// Done swaps the panel between them: both are the same secret in
    /// two forms and neither belongs on the Result.
    fn lightning_secret(&self, c: &Chrome<'_>, which: SecretKind) -> Node {
        let s = self.strings();
        let Some(flow) = self.lightning.as_ref() else {
            return screens::menu(c, s.lightning_secret_title, None, Vec::new(), Vec::new());
        };
        let Some(Answer::Node(node)) = flow.answer() else {
            return screens::menu(c, s.lightning_secret_title, None, Vec::new(), Vec::new());
        };
        let (title, value) = match which {
            SecretKind::NodeKey => (
                s.lightning_secret_title,
                text::hex(node.private.expose().as_slice()),
            ),
            SecretKind::Entropy => (
                s.lightning_entropy_title,
                match &node.seed {
                    Some(cipher) => text::hex(cipher.entropy.expose().as_slice()),
                    None => String::new(),
                },
            ),
        };
        let secondary = node.seed.as_ref().map(|_| match which {
            SecretKind::NodeKey => Action::new(ids::LIGHTNING_ENTROPY, s.lightning_entropy_title),
            SecretKind::Entropy => {
                Action::new(ids::LIGHTNING_NODE_SECRET, s.lightning_secret_title)
            }
        });
        screens::secret(
            c,
            SecretScreen {
                title,
                value: Value::Text(&value),
                revealed: self.revealed(ids::LIGHTNING_REVEAL),
                panel: ids::LIGHTNING_REVEAL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
                rows: Vec::new(),
                action: Some(Action::new(ids::LIGHTNING_DONE, s.action_done)),
                secondary,
                pager: None,
            },
        )
    }

    /// §5 Choice, "From?": the two sources, with the loaded-key row
    /// dimmed and the reason beside it where no loaded key has BIP-39
    /// words (§4.11).
    pub(crate) fn lightning_from_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let items = vec![
            Item::chosen(
                ids::at(ids::PICK_BASE, 0),
                s.lightning_from_aezeed,
                picked == 0,
            ),
            if self.lightning_keys() == 0 {
                Item::dimmed(s.lightning_from_key, String::from(s.lightning_no_words))
            } else {
                Item::chosen(
                    ids::at(ids::PICK_BASE, 1),
                    s.lightning_from_key,
                    picked == 1,
                )
            },
        ];
        screens::choice(
            c,
            s.lightning_from_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }
}

/// The line a refused cipher seed puts on the Result.
fn refusal(e: osk_bip::aezeed::Error, s: &crate::strings::Strings) -> &'static str {
    use osk_bip::aezeed::Error;
    match e {
        Error::Version(_) => s.lightning_bad_version,
        Error::Checksum => s.lightning_bad_checksum,
        Error::Passphrase | Error::Kdf => s.lightning_bad_passphrase,
    }
}
