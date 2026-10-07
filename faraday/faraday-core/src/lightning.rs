//! A Lightning node's key (`docs/OPENSIGNER-PARITY.md` plan C3): from a
//! loaded key, read the way ldk-node reads a BIP-39 seed, or from an LND
//! cipher seed (aezeed) typed here. The arithmetic is `osk-bip::aezeed`'s,
//! as OpenSigner's own Lightning tool uses it. The node's public key is
//! public; its private key is a secret, into the vault or out only
//! through the secret sheet.

use faraday_vault::Record;
use faraday_vault::records::{field, kind};
use osk_bip::aezeed;
use osk_bip::bip39::{Language, Mnemonic};
use osk_crypto::Secret;
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
pub struct Node {
    /// The node's public key, compressed: its node id.
    pub public: [u8; 33],
    /// Its private key.
    pub private: Zeroizing<[u8; 32]>,
    /// The day an aezeed says it was made.
    pub birthday: Option<(i32, u32, u32)>,
}

/// What the Lightning screen holds.
pub struct LightningState {
    /// The source chosen.
    pub source: Option<NodeSource>,
    /// The aezeed's words, as typed.
    pub typed: Zeroizing<String>,
    /// The aezeed's passphrase; empty is LND's default.
    pub passphrase: Zeroizing<String>,
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

fn node_of(key: &osk_bip::keys::DerivedKey, birthday: Option<(i32, u32, u32)>) -> Node {
    let public =
        osk_bip::bitcoin::secp256k1::PublicKey::from_secret_key(key.secp(), key.secret_key())
            .serialize();
    Node {
        public,
        private: Zeroizing::new(key.secret_key().secret_bytes()),
        birthday,
    }
}

impl Faraday {
    /// Opens the screen.
    pub(crate) fn lightning_open(&mut self) {
        self.lightning = Some(LightningState {
            source: None,
            typed: Zeroizing::new(String::new()),
            passphrase: Zeroizing::new(String::new()),
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
        let net = self.session.network();
        let Some(l) = self.lightning.as_ref() else {
            return;
        };
        let found: Result<Node, String> = match l.source {
            Some(NodeSource::Key(fp)) => {
                let key = self
                    .session
                    .keys
                    .iter()
                    .find(|k| k.master.fingerprint().0 == fp);
                match key.and_then(|k| Some((k, k.words.as_ref()?))) {
                    None => Err("This key has no BIP-39 seed".to_string()),
                    Some((k, words)) => Mnemonic::parse(k.language, words)
                        .map_err(|e| e.to_string())
                        .and_then(|m| {
                            let p = k.passphrase.as_deref().map_or("", |p| p.as_str());
                            m.to_seed(p.as_bytes()).map_err(|e| e.to_string())
                        })
                        .map(|seed| node_of(&aezeed::ldk_node_key(&seed, net), None)),
                }
            }
            Some(NodeSource::Aezeed) => {
                let mut idx = [0u16; aezeed::NUM_WORDS];
                let words: Vec<&str> = l.typed.split_whitespace().collect();
                if words.len() != aezeed::NUM_WORDS {
                    Err(format!("An aezeed is 24 words; {} are typed", words.len()))
                } else if let Some(k) = words
                    .iter()
                    .position(|w| Language::English.index_of(w).is_none())
                {
                    Err(format!("Word {} is not on the list", k + 1))
                } else {
                    for (slot, w) in idx.iter_mut().zip(&words) {
                        *slot = Language::English.index_of(w).unwrap_or(0);
                    }
                    let r = aezeed::decode(&idx, l.passphrase.as_bytes())
                        .map_err(|e| format!("Not an aezeed for this passphrase: {e:?}"))
                        .map(|seed| {
                            let birthday = aezeed::birthday_date(seed.birthday);
                            let mut entropy = Secret::new([0u8; aezeed::ENTROPY_SIZE]);
                            entropy
                                .expose_mut()
                                .copy_from_slice(seed.entropy().expose());
                            node_of(&aezeed::node_key(&entropy, net), Some(birthday))
                        });
                    zeroize::Zeroize::zeroize(&mut idx);
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

    fn lightning_secret_text(&self) -> Option<(String, Zeroizing<String>)> {
        let n = self.lightning.as_ref()?.node.as_ref()?;
        let id: String = n.public.iter().map(|b| format!("{b:02x}")).collect();
        let private: String = n.private.iter().map(|b| format!("{b:02x}")).collect();
        Some((id, Zeroizing::new(private)))
    }

    /// One press on the screen.
    pub(crate) fn lightning_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        match action {
            A::Lightning => return self.lightning_open(),
            A::LResolve => return self.lightning_resolve(),
            A::LVault => {
                if self.vaults.open.get(self.vaults.current).is_none() {
                    return self.toast("No vault is open");
                }
                let Some((id, private)) = self.lightning_secret_text() else {
                    return;
                };
                let note = Zeroizing::new(format!(
                    "Lightning node key\nnode id {id}\nprivate key {}",
                    private.as_str()
                ));
                self.vault_push(
                    Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
                    "The node key is in the vault",
                );
                return;
            }
            A::LOut => {
                let Some((id, private)) = self.lightning_secret_text() else {
                    return;
                };
                self.offer_secret(crate::secrets::SecretOut {
                    name: format!("lightning-{}.txt", &id[..16]),
                    bytes: Zeroizing::new(
                        format!("node id {id}\nprivate key {}\n", private.as_str()).into_bytes(),
                    ),
                    what: "A Lightning node's private key",
                    gives: "Whoever has it is the node: its channels and their money",
                    round: None,
                });
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

    /// Typing the aezeed and its passphrase. Returns whether the key was
    /// taken.
    pub(crate) fn lightning_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
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
