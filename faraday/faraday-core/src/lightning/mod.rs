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
}

impl Faraday {
    /// Opens the screen.
    pub(crate) fn lightning_open(&mut self) {
        self.lightning = Some(LightningState {
            source: None,
            typed: crate::secret_text::SecretText::new(),
            passphrase: crate::secret_text::SecretText::new(),
            on_passphrase: false,
            node: None,
            error: None,
            shown: false,
            back: self.screen,
        });
        self.screen = Screen::Lightning;
    }

    /// Finds the node key for the source chosen.
    fn lightning_resolve(&mut self) {
        let c = self.compute();
        let net = self.session.network();
        let Some(l) = self.lightning.as_ref() else {
            return;
        };
        let found: Result<Node, String> = match l.source {
            Some(NodeSource::Key(fp)) => {
                match self
                    .session
                    .keys
                    .position_of(fp)
                    .and_then(|k| self.session.keys.lightning_node(&c, k))
                {
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
                    Err(fill(EN.lightning_word_is_not, &[&(k + 1).to_string()]))
                } else {
                    for (slot, w) in idx.iter_mut().zip(&words) {
                        *slot = Language::English.index_of(w).unwrap_or(0);
                    }
                    let r = faraday_keys::lightning::node_from_aezeed(
                        &c,
                        &idx,
                        l.passphrase.as_bytes(),
                        net,
                    )
                    .map_err(|e| fill(EN.lightning_not_an_aezeed, &[&e]));
                    faraday_keys::lightning::wipe_words(&mut idx);
                    r
                }
            }
            None => return,
        };
        if let Some(l) = self.lightning.as_mut() {
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
    fn lightning_secret_text(
        &self,
        why: faraday_keys::Reveal,
    ) -> Option<(String, Zeroizing<String>)> {
        let n = self.lightning.as_ref()?.node.as_ref()?;
        let id: String = n.public().iter().map(|b| format!("{b:02x}")).collect();
        Some((id, n.private_hex(why)?))
    }

    /// One press on the screen.
    pub(crate) fn lightning_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        match action {
            A::Lightning => return self.lightning_open(),
            A::LResolve => return self.lightning_resolve(),
            A::LVault => {
                if self.vaults.open.get(self.vaults.current).is_none() {
                    return self.toast(EN.lightning_no_vault);
                }
                let Some((id, private)) = self.lightning_secret_text(
                    faraday_keys::Reveal::VaultEntry(faraday_keys::Part::NodeKey),
                ) else {
                    return;
                };
                let note = fill_secret(EN.lightning_lightning, &[&id, private.as_str()]);
                self.vault_push(
                    Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
                    EN.lightning_the_node,
                );
                return;
            }
            A::LOut => {
                let Some((id, private)) = self.lightning_secret_text(
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
                return;
            }
            _ => {}
        }
        let Some(l) = self.lightning.as_mut() else {
            return;
        };
        match action {
            A::LKey(fp) => {
                l.source = Some(NodeSource::Key(fp));
                l.node = None;
                l.error = None;
                self.lightning_resolve();
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
    pub(crate) fn lightning_reads_aezeed(&self) -> bool {
        self.sheet.is_none()
            && self
                .lightning
                .as_ref()
                .is_some_and(|l| l.source == Some(NodeSource::Aezeed))
    }

    /// Typing the aezeed and its passphrase. Returns whether the key was
    /// taken.
    pub(crate) fn lightning_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        if self.screen != Screen::Lightning {
            return false;
        }
        let Some(l) = self.lightning.as_mut() else {
            return false;
        };
        if key == K::Escape {
            self.screen = l.back;
            self.lightning = None;
            return true;
        }
        if l.source != Some(NodeSource::Aezeed) {
            return false;
        }
        let field = if l.on_passphrase {
            &mut l.passphrase
        } else {
            &mut l.typed
        };
        match key {
            K::Char(c) if l.on_passphrase && (' '..='~').contains(&c) => field.push(c),
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
                self.lightning_resolve();
                return true;
            }
            _ => return false,
        }
        if let Some(l) = self.lightning.as_mut() {
            l.node = None;
        }
        true
    }
}
