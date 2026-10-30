//! A Lightning node's key: from a
//! loaded key, read the way ldk-node reads a BIP-39 seed, or from an LND
//! cipher seed (aezeed) typed here. The arithmetic is `faraday-bip::aezeed`'s,
//! as the Lightning tool uses it. The node's public key is
//! public; its private key is a secret, into the vault or out only
//! through the secret sheet.

pub(crate) mod screen;

use crate::strings::fill;
use crate::strings::{EN, fill_secret};
use faraday_bip::bip39::Language;
use faraday_vault::Record;
use faraday_vault::records::{field, kind};
use zeroize::Zeroizing;

use crate::{Faraday, Screen};
use faraday_crypto::work::Compute;
use faraday_keys::Keys;

/// Where the node key comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeSource {
    /// A loaded key, by fingerprint, read as ldk-node reads its seed.
    Key([u8; 4]),
    /// An LND cipher seed typed here.
    Aezeed,
}

/// The node key, once found.
pub use faraday_keys::lightning::Node;

/// What the Lightning screen holds.
pub struct LightningState {
    /// The source chosen.
    pub source: Option<NodeSource>,
    /// The aezeed's words, as typed.
    pub typed: crate::secret_text::SecretText,
    /// The aezeed's passphrase; empty is LND's default.
    pub passphrase: crate::secret_text::SecretText,
    /// Typing goes to the passphrase.
    pub on_passphrase: bool,
    /// The node key, once found.
    pub node: Option<Node>,
    /// Why it was not.
    pub error: Option<String>,
    /// The private key is on screen.
    pub shown: bool,
    /// Where it returns to.
    pub back: Screen,
    /// Use a sample on the aezeed's words (`docs/MODES.md` §6).
    pub(crate) sample: crate::samples::put::Use,
}

/// The Lightning screen's area: its state while it is open.
#[derive(Default)]
pub(crate) struct LightningArea {
    state: Option<LightningState>,
}

impl LightningArea {
    /// The screen's state, while it is open.
    pub(crate) fn state(&self) -> Option<&LightningState> {
        self.state.as_ref()
    }

    /// Opens the screen, to return to `back`.
    pub(crate) fn open(&mut self, back: Screen) {
        self.state = Some(LightningState {
            source: None,
            typed: crate::secret_text::SecretText::new(),
            passphrase: crate::secret_text::SecretText::new(),
            on_passphrase: false,
            node: None,
            error: None,
            shown: false,
            back,
            sample: Default::default(),
        });
    }

    /// Forgets the screen's state: the words, the passphrase and the key.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }

    /// Costs a key derivation, and an aezeed's scrypt. Finds the node key
    /// for the source chosen.
    pub(crate) fn resolve(&mut self, c: &Compute, keys: &Keys) {
        let Some(l) = self.state.as_ref() else {
            return;
        };
        let found: Result<Node, String> = match l.source {
            Some(NodeSource::Key(fp)) => {
                match keys.position_of(fp).and_then(|k| keys.lightning_node(c, k)) {
                    None => Err(EN.lightning_this_key.to_string()),
                    Some(node) => node,
                }
            }
            Some(NodeSource::Aezeed) => {
                let mut idx = [0u16; faraday_keys::lightning::AEZEED_WORDS];
                let words: Vec<&str> = l.typed.split_whitespace().collect();
                if words.len() != faraday_keys::lightning::AEZEED_WORDS {
                    Err(fill(EN.lightning_an_aezeed, &[&words.len().to_string()]))
                } else if let Some(k) = words
                    .iter()
                    .position(|w| Language::English.index_of(w).is_none())
                {
                    Err(fill(EN.lightning_word_not_in_list, &[&(k + 1).to_string()]))
                } else {
                    for (slot, w) in idx.iter_mut().zip(&words) {
                        *slot = Language::English.index_of(w).unwrap_or(0);
                    }
                    let r = keys
                        .node_from_aezeed(c, &idx, l.passphrase.as_bytes())
                        .map_err(|e| fill(EN.lightning_not_an_aezeed, &[&e]));
                    faraday_keys::lightning::wipe_words(&mut idx);
                    r
                }
            }
            None => return,
        };
        if let Some(l) = self.state.as_mut() {
            match found {
                Ok(n) => {
                    l.node = Some(n);
                    l.error = None;
                }
                Err(e) => l.error = Some(e),
            }
        }
    }

    /// The node id and the private key, for `why`.
    pub(crate) fn secret_text(
        &self,
        why: faraday_keys::Reveal,
    ) -> Option<(String, Zeroizing<String>)> {
        let n = self.state.as_ref()?.node.as_ref()?;
        let id: String = n.public().iter().map(|b| format!("{b:02x}")).collect();
        Some((id, n.private_hex(why)?))
    }

    /// Costs a key derivation when a loaded key is chosen. A press that
    /// changes only this screen: a source chosen, the passphrase field, the
    /// private key shown.
    pub(crate) fn act(&mut self, c: &Compute, keys: &Keys, action: crate::Action) {
        use crate::Action as A;
        let Some(l) = self.state.as_mut() else {
            return;
        };
        match action {
            A::LKey(fp) => {
                l.source = Some(NodeSource::Key(fp));
                l.node = None;
                l.error = None;
                self.resolve(c, keys);
            }
            A::LAezeed => {
                l.source = Some(NodeSource::Aezeed);
                l.node = None;
                l.error = None;
            }
            A::LPassphrase => l.on_passphrase = !l.on_passphrase,
            A::LShow => l.shown = !l.shown,
            _ => {}
        }
    }

    /// Whether the screen reads a typed aezeed, whose Enter stretches it
    /// with scrypt.
    pub(crate) fn reads_aezeed(&self, no_sheet: bool) -> bool {
        no_sheet
            && self
                .state
                .as_ref()
                .is_some_and(|l| l.source == Some(NodeSource::Aezeed))
    }

    /// Costs the aezeed's scrypt on Enter. Typing the aezeed and its
    /// passphrase, on `screen`. Returns whether the key was taken. Escape leaves the screen for where it came from.
    pub(crate) fn key(
        &mut self,
        key: faraday_shell_api::Key,
        screen: &mut Screen,
        c: &Compute,
        keys: &Keys,
    ) -> bool {
        use faraday_shell_api::Key as K;
        if *screen != Screen::Lightning {
            return false;
        }
        let Some(l) = self.state.as_mut() else {
            return false;
        };
        if key == K::Escape {
            *screen = l.back;
            self.state = None;
            return true;
        }
        if l.source != Some(NodeSource::Aezeed) {
            return false;
        }
        l.sample.dismiss();
        let field = if l.on_passphrase {
            &mut l.passphrase
        } else {
            &mut l.typed
        };
        match key {
            K::Char(c) if l.on_passphrase && (' '..='~').contains(&c) => field.push(c),
            // Anything else is refused, and the screen says so.
            K::Char(c) if l.on_passphrase && !c.is_control() => {
                l.error = Some(crate::state::ascii_refused());
                return true;
            }
            K::Char(c) if c.is_ascii_alphabetic() => field.push(c.to_ascii_lowercase()),
            K::Char(' ') => {
                if !field.is_empty() && !field.ends_with(' ') {
                    field.push(' ');
                }
            }
            K::Backspace => {
                field.pop();
            }
            K::Tab => l.on_passphrase = !l.on_passphrase,
            K::Enter => {
                l.on_passphrase = false;
                self.resolve(c, keys);
                return true;
            }
            _ => return false,
        }
        if let Some(l) = self.state.as_mut() {
            l.node = None;
        }
        true
    }

    /// Practice's passphrase typed into the aezeed's passphrase field.
    pub(crate) fn set_passphrase(&mut self, text: &str) {
        if let Some(l) = self.state.as_mut() {
            l.passphrase.set(text);
            l.node = None;
            l.error = None;
        }
    }

    /// Whether the aezeed's words field draws Use a sample: the cipher
    /// seed chosen, nothing typed or pressed.
    pub(crate) fn sample_shown(&self) -> bool {
        self.state.as_ref().is_some_and(|l| {
            l.source == Some(NodeSource::Aezeed) && l.typed.is_empty() && l.sample.shown()
        })
    }

    /// An aezeed read by a scanner, typed in: the cipher seed chosen as
    /// the source, its words lowercased with one space between, the
    /// passphrase left for the person to type (`docs/SCANNING.md` §4.2).
    pub(crate) fn put_aezeed(&mut self, words: &str) {
        if let Some(l) = self.state.as_mut() {
            let typed = Zeroizing::new(
                words
                    .split_whitespace()
                    .map(str::to_lowercase)
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            l.source = Some(NodeSource::Aezeed);
            l.typed.set(&typed);
            l.node = None;
            l.error = None;
            l.sample.dismiss();
        }
    }

    /// Types the sample cipher seed's `words` in.
    pub(crate) fn use_sample(&mut self, words: &str) {
        if let Some(l) = self.state.as_mut() {
            l.typed.set(words);
            l.node = None;
            l.error = None;
            l.sample.fill();
        }
    }
}

impl Faraday {
    /// The Lightning screen's state, while it is open.
    pub fn lightning(&self) -> Option<&LightningState> {
        self.lightning.state()
    }

    /// One press on the screen.
    pub(crate) fn lightning_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        match action {
            A::Lightning => {
                self.lightning.open(self.screen);
                self.screen = Screen::Lightning;
            }
            A::LResolve => {
                let c = self.compute();
                self.lightning.resolve(&c, &self.session.keys);
            }
            A::LVault => {
                if self.vaults.open.get(self.vaults.current).is_none() {
                    return self.toast(EN.lightning_no_vault);
                }
                let Some((id, private)) = self.lightning.secret_text(
                    faraday_keys::Reveal::VaultEntry(faraday_keys::Part::NodeKey),
                ) else {
                    return;
                };
                let note = fill_secret(EN.lightning_lightning, &[&id, private.as_str()]);
                self.vault_push(
                    Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
                    EN.lightning_the_node,
                );
            }
            A::LOut => {
                let Some((id, private)) = self.lightning.secret_text(
                    faraday_keys::Reveal::OutboxFile(faraday_keys::Part::NodeKey),
                ) else {
                    return;
                };
                self.offer_secret(crate::secrets::SecretOut::new(
                    format!("lightning-{}.txt", &id[..16]),
                    Zeroizing::new(
                        format!("node id {id}\nprivate key {}\n", private.as_str()).into_bytes(),
                    ),
                    EN.lightning_a_lightning,
                    EN.lightning_whoever_has,
                ));
            }
            _ => {
                let c = self.compute();
                self.lightning.act(&c, &self.session.keys, action);
            }
        }
    }

    /// The read card's **Use in Lightning**: the screen opens, to return
    /// to the screen under the scanner, with the aezeed read typed in.
    pub(crate) fn lightning_with_aezeed(&mut self, words: &str) {
        self.lightning.open(self.screen);
        self.screen = Screen::Lightning;
        self.lightning.put_aezeed(words);
    }

    /// Whether the screen reads a typed aezeed, whose Enter stretches it
    /// with scrypt.
    pub(crate) fn lightning_reads_aezeed(&self) -> bool {
        self.lightning.reads_aezeed(self.sheet.is_none())
    }

    /// Whether `key` is left for the tick after a frame: Enter where it
    /// reads an aezeed, which scrypt stretches for seconds on a Pi.
    pub(crate) fn key_slow(&self, key: faraday_shell_api::Key) -> bool {
        key == faraday_shell_api::Key::Enter
            && self.screen == Screen::Lightning
            && self.lightning_reads_aezeed()
    }

    /// Typing the aezeed and its passphrase. Returns whether the key was
    /// taken.
    pub(crate) fn lightning_key(&mut self, key: faraday_shell_api::Key) -> bool {
        let c = self.compute();
        self.lightning
            .key(key, &mut self.screen, &c, &self.session.keys)
    }
}
