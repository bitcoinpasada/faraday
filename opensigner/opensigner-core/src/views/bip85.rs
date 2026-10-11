//! BIP-85's other applications, from the key page's "BIP-85" row
//! (`docs/PLANNING.md` §16.114).
//!
//! Words opens the child-key flow, which adds a key. The other five
//! derive a value to transcribe: a password's length and then the index
//! on the digits pad, and the value on a §5 Secret screen with the eye.
//! Nothing is kept, loaded or exported, and the value is derived where
//! it is drawn rather than held beside the screen.

use alloc::string::String;
use alloc::vec::Vec;

use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Chrome, Field, Pad, PadKind, Secret as SecretScreen, Value};

use crate::{Bip85App, Bip85Step, OpenSigner, ids, strings, text};

impl OpenSigner {
    /// The three screens of one application: the length, the index and
    /// the value.
    pub(crate) fn view_bip85(&self, key: usize) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            let Some(flow) = self.bip85.as_ref() else {
                return screens::menu(c, s.key_bip85_row, None, Vec::new(), Vec::new());
            };
            if self.keys.get(key).is_none() {
                return screens::menu(c, s.key_bip85_row, None, Vec::new(), Vec::new());
            }
            match flow.step {
                Bip85Step::Length => self.bip85_length(c),
                Bip85Step::Index => self.bip85_index(c),
                Bip85Step::Value => self.bip85_value(c, key),
            }
        })
    }

    /// §5 Pad, "Length": a password's length on the digits pad, the
    /// bounds the application takes on the caption line, and ✓ live only
    /// while what is typed is one of them.
    fn bip85_length(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let Some(flow) = self.bip85.as_ref() else {
            return screens::menu(c, s.bip85_length_title, None, Vec::new(), Vec::new());
        };
        let caption = flow.app.lengths().map(|(low, high)| {
            strings::fill(
                s.bip85_length_range,
                &[&alloc::format!("{low}"), &alloc::format!("{high}")],
            )
        });
        screens::pad(
            c,
            Pad {
                title: String::from(s.bip85_length_title),
                id: ids::BIP85_LENGTH_PAD,
                kind: PadKind::Pin {
                    scramble: None,
                    done: flow.length_value().is_some(),
                },
                field: Field::Count(flow.length.clone()),
                progress: None,
                entries: None,
                words: None,
                eye: None,
                caption,
                error: None,
                action: None,
            },
        )
    }

    /// §5 Pad, "Index": which child of the application this is, on the
    /// same pad the child flow asks its index on.
    fn bip85_index(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let Some(flow) = self.bip85.as_ref() else {
            return screens::menu(c, s.open_child_index, None, Vec::new(), Vec::new());
        };
        screens::pad(
            c,
            Pad {
                title: String::from(s.open_child_index),
                id: ids::BIP85_INDEX_PAD,
                kind: PadKind::Pin {
                    scramble: None,
                    done: flow.index_value().is_some(),
                },
                field: Field::Count(flow.index.clone()),
                progress: None,
                entries: None,
                words: None,
                eye: None,
                caption: Some(String::from(s.open_child_index_range)),
                error: None,
                action: None,
            },
        )
    }

    /// §5 Secret: the derived value whole, masked until a finger is on
    /// the panel or the eye is running, titled by the application and
    /// the index. §4.10 keeps a secret off the clipboard, so the screen
    /// offers Done and nothing else.
    fn bip85_value(&self, c: &Chrome<'_>, key: usize) -> Node {
        let s = self.strings();
        let Some(flow) = self.bip85.as_ref() else {
            return screens::menu(c, s.key_bip85_row, None, Vec::new(), Vec::new());
        };
        let title = strings::fill(
            s.bip85_value_title,
            &[app_name(flow.app, s), &flow.index.clone()],
        );
        let value = self.bip85_derived(key).unwrap_or_default();
        screens::secret(
            c,
            SecretScreen {
                title: &title,
                value: Value::Text(&value),
                revealed: self.revealed(ids::BIP85_REVEAL),
                panel: ids::BIP85_REVEAL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
                rows: Vec::new(),
                action: Some(Action::new(ids::BIP85_DONE, s.action_done)),
                secondary: None,
                pager: None,
            },
        )
    }

    /// The value the flow's application, parameters and index derive
    /// from the key's master, as the text the panel shows. It is
    /// computed here and kept nowhere: leaving the screen is the end of
    /// it (§16.114).
    pub(crate) fn bip85_derived(&self, key: usize) -> Option<String> {
        let flow = self.bip85.as_ref()?;
        let master = self.keys.get(key)?.master.as_ref()?;
        let index = flow.index_value()?;
        match flow.app {
            // Words is the child-key flow and adds a key; it never
            // reaches this screen.
            Bip85App::Words => None,
            Bip85App::Wif => Some(String::from(
                osk_bip::bip85::child_wif(master, index).ok()?.as_str(),
            )),
            Bip85App::Xprv => Some(String::from(
                osk_bip::bip85::child_xprv(master, index).ok()?.as_str(),
            )),
            Bip85App::Hex => {
                let bytes = osk_bip::bip85::child_hex(master, flow.bytes, index).ok()?;
                Some(text::hex(bytes.as_bytes()))
            }
            Bip85App::Base64 => {
                let length = flow.length_value()?;
                let pwd = osk_bip::bip85::child_password_base64(master, length, index).ok()?;
                Some(String::from(pwd.as_str()))
            }
            Bip85App::Base85 => {
                let length = flow.length_value()?;
                let pwd = osk_bip::bip85::child_password_base85(master, length, index).ok()?;
                Some(String::from(pwd.as_str()))
            }
        }
    }
}

/// What an application is called, on its Choice row and in the title of
/// the value it derives.
pub(crate) fn app_name(app: Bip85App, s: &crate::strings::Strings) -> &'static str {
    match app {
        Bip85App::Words => s.bip85_app_words,
        Bip85App::Wif => s.bip85_app_wif,
        Bip85App::Xprv => s.bip85_app_xprv,
        Bip85App::Hex => s.bip85_app_hex,
        Bip85App::Base64 => s.bip85_app_base64,
        Bip85App::Base85 => s.bip85_app_base85,
    }
}
