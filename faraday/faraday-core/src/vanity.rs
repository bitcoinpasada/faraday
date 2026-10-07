//! Vanity addresses (`docs/OPENSIGNER-PARITY.md` C4): a loaded key's
//! first address made to begin with characters the person chose, by
//! turning one of the key's two dials, the account index or a suffix on
//! its BIP-39 passphrase. The engine is upstream's (`osk_bip::vanity`)
//! and so is the state between frames (`opensigner_core::vanity::Grind`):
//! every tick tests a budget of candidates and comes back, so the screen
//! stays live and Stop lands. A find is a fact about the key, the same on
//! any device that turns the same dial.

use opensigner_core::vanity::{Dial, Grind, Step};
use osk_bip::bip39::Mnemonic;
use osk_bip::keys::{Fingerprint, ScriptType};
use osk_bip::vanity::{self as engine, Key, Method};
use zeroize::Zeroizing;

use crate::create::NewKind;
use crate::{Faraday, Screen, flow};

/// The cards, in order.
pub mod vstep {
    /// The key.
    pub const KEY: u8 = 0;
    /// Which dial turns.
    pub const DIAL: u8 = 1;
    /// Which kind of address.
    pub const SCRIPT: u8 = 2;
    /// The characters.
    pub const PREFIX: u8 = 3;
    /// The search and what it found.
    pub const RUN: u8 = 4;
    /// How many.
    pub const COUNT: usize = 5;
}

/// Everything the vanity screen can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanityAction {
    /// Open the flow.
    Open,
    /// Open or close a card.
    Step(u8),
    /// Done with a card.
    Next(u8),
    /// The key, by fingerprint.
    Key([u8; 4]),
    /// The dial, by its place in `Dial::ALL`.
    Dial(u8),
    /// The kind of address, by its place in `ScriptType::ALL`.
    Script(u8),
    /// Start searching.
    Start,
    /// Stop searching.
    Stop,
    /// Show the passphrase a find names, or hide it.
    Show,
    /// Open the wallet the find's address belongs to: the key at the
    /// account found, or the key with the passphrase found.
    Use,
}

/// The vanity screen's state.
pub struct VanityState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; vstep::COUNT],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The key, by fingerprint.
    pub key: Option<[u8; 4]>,
    /// The search, upstream's own state.
    pub grind: Grind,
    /// A search is under way.
    pub running: bool,
    /// The passphrase a find names is on screen.
    pub shown: bool,
    /// Where the flow returns to.
    pub back: Screen,
    /// The last refusal.
    pub error: Option<String>,
}

/// A script type's name, as the cards say it, with the characters its
/// addresses begin with on `net`.
pub fn script_name(s: ScriptType, net: osk_bip::keys::Network) -> String {
    let name = match s {
        ScriptType::Legacy => "Legacy",
        ScriptType::NestedSegwit => "Nested SegWit",
        ScriptType::NativeSegwit => "Native SegWit",
        ScriptType::Taproot => "Taproot",
    };
    match engine::fixed_prefix(s, net) {
        "" => format!("{name} · m… or n…"),
        p => format!("{name} · {p}…"),
    }
}

/// A dial's name and what it changes.
pub fn dial_name(d: Dial) -> (&'static str, &'static str) {
    match d {
        Dial::Account => (
            "Account number",
            "The same words and passphrase, a different account: fast. The wallet is the key's \
             own, at the account found.",
        ),
        Dial::Passphrase => (
            "Passphrase",
            "A few characters added to the key's BIP-39 passphrase: slow, one PBKDF2 a try. \
             The words with that passphrase are another wallet, which needs the passphrase \
             kept with them.",
        ),
    }
}

impl Faraday {
    fn vanity_open(&mut self) {
        if self.session.keys.is_empty() {
            self.toast("Load a key first");
            return;
        }
        let key =
            (self.session.keys.len() == 1).then(|| self.session.keys[0].master.fingerprint().0);
        let net = self.session.network();
        self.vanity = Some(VanityState {
            open: Some(if key.is_some() {
                vstep::DIAL
            } else {
                vstep::KEY
            }),
            done: [key.is_some(), false, false, false, false],
            scroll: flow::Scroll::default(),
            key,
            grind: Grind::new(Dial::Account, ScriptType::NativeSegwit, net),
            running: false,
            shown: false,
            back: self.screen,
            error: None,
        });
        self.screen = Screen::Vanity;
    }

    /// The key chosen, from the session.
    fn vanity_key(&self) -> Option<&crate::wallet::Key> {
        let fp = self.vanity.as_ref()?.key?;
        self.session
            .keys
            .iter()
            .find(|k| k.master.fingerprint().0 == fp)
    }

    /// Whether the chosen key can turn the passphrase dial: only a key
    /// loaded from BIP-39 words can.
    pub fn vanity_has_words(&self) -> bool {
        self.vanity_key().is_some_and(|k| k.words.is_some())
    }

    /// Runs one of the screen's actions.
    pub(crate) fn vanity_act(&mut self, a: VanityAction) {
        use VanityAction as V;
        if a == V::Open {
            return self.vanity_open();
        }
        if a == V::Use {
            return self.vanity_use();
        }
        let net = self.session.network();
        let now = self.now_ms;
        let words = self.vanity_has_words();
        let Some(v) = self.vanity.as_mut() else {
            return;
        };
        v.error = None;
        let open = |v: &mut VanityState, s: u8| {
            v.open = Some(s);
            v.scroll.follow = true;
        };
        match a {
            V::Step(s) => {
                if v.open == Some(s) {
                    v.open = None;
                } else if (0..s).all(|i| v.done[usize::from(i)]) {
                    open(v, s);
                }
            }
            V::Next(s) => {
                if s == vstep::PREFIX && !v.grind.prefix.has_free(v.grind.script, net) {
                    v.error = Some("Type at least one character after the fixed part".into());
                    return;
                }
                v.done[usize::from(s)] = true;
                let next = s + 1;
                if usize::from(next) < vstep::COUNT {
                    open(v, next);
                }
            }
            V::Key(fp) => {
                v.key = Some(fp);
                v.done[usize::from(vstep::KEY)] = true;
                v.running = false;
                v.grind.find = None;
                open(v, vstep::DIAL);
            }
            V::Dial(i) => {
                if let Some(&d) = Dial::ALL.get(usize::from(i)) {
                    if d == Dial::Passphrase && !words {
                        v.error = Some(
                            "This key was not loaded from BIP-39 words: only its account turns"
                                .into(),
                        );
                        return;
                    }
                    v.grind.dial = d;
                    v.grind.find = None;
                    v.running = false;
                }
            }
            V::Script(i) => {
                if let Some(&s) = ScriptType::ALL.get(usize::from(i)) {
                    v.grind.set_script(s, net);
                    v.grind.find = None;
                    v.running = false;
                    v.done[usize::from(vstep::PREFIX)] = false;
                }
            }
            V::Start => {
                if v.grind.prefix.has_free(v.grind.script, net) {
                    v.grind.start(now);
                    v.running = true;
                    v.shown = false;
                }
            }
            V::Stop => {
                v.running = false;
                if v.grind.find.is_none() {
                    v.grind.step = Step::Prefix;
                }
            }
            V::Show => v.shown = !v.shown,
            V::Open | V::Use => {}
        }
    }

    /// One tick of a search under way: a budget of candidates, the rate
    /// measured, the find kept. Returns whether to draw.
    pub(crate) fn vanity_tick(&mut self, now_ms: u64) -> bool {
        if self.screen != Screen::Vanity || !self.vanity.as_ref().is_some_and(|v| v.running) {
            return false;
        }
        let net = self.session.network();
        let Some(key) = self.vanity_key() else {
            return false;
        };
        let Some(v) = self.vanity.as_ref() else {
            return false;
        };
        let g = &v.grind;
        let outcome = match g.dial {
            Dial::Account => engine::grind(
                &Key::Master(&key.master),
                &Method::Account,
                g.script,
                g.prefix.as_str(),
                net,
                g.cursor,
                g.budget,
            ),
            Dial::Passphrase => {
                let Some(words) = key.words.as_ref() else {
                    return false;
                };
                let Ok(m) = Mnemonic::parse(key.language, words) else {
                    return false;
                };
                let base: &[u8] = key.passphrase.as_ref().map_or(&[], |p| p.as_bytes());
                engine::grind(
                    &Key::Words(&m),
                    &Method::Passphrase { base },
                    g.script,
                    g.prefix.as_str(),
                    net,
                    g.cursor,
                    g.budget,
                )
            }
        };
        let Some(v) = self.vanity.as_mut() else {
            return false;
        };
        v.grind.ticked(outcome.tested, now_ms);
        if let Some(find) = outcome.find {
            v.grind.find = Some(find);
            v.grind.step = Step::Found;
            v.running = false;
        } else if v.grind.exhausted {
            v.running = false;
            v.error = Some("Every counter was tried: no address begins that way".into());
        }
        self.dirty = true;
        self.commands.push_back(osk_shell_api::Command::Draw);
        true
    }

    /// The wallet the find belongs to, opened: on the account dial the
    /// key's own wallet at the account found; on the passphrase dial the
    /// words loaded again with the passphrase found, and that key's
    /// wallet. The passphrase is a secret, kept with the key in the
    /// session; saving it to a vault is the vault's own choice.
    fn vanity_use(&mut self) {
        let Some(key) = self.vanity_key() else {
            return;
        };
        let Some(v) = self.vanity.as_ref() else {
            return;
        };
        let Some(find) = v.grind.find.as_ref() else {
            return;
        };
        let script = v.grind.script;
        let label = key.label.clone();
        let (master_fp, account) = match v.grind.dial {
            Dial::Account => (key.master.fingerprint(), find.account),
            Dial::Passphrase => {
                let Some(words) = key.words.clone() else {
                    return;
                };
                let base = key
                    .passphrase
                    .as_ref()
                    .map_or(String::new(), |p| p.to_string());
                let passphrase = Zeroizing::new(format!("{base}{}", find.suffix.as_str()));
                let lang = key.language;
                let Ok(m) = Mnemonic::parse(lang, &words) else {
                    return;
                };
                match self.session.add_mnemonic(
                    &m,
                    &passphrase,
                    &format!("{label} · vanity passphrase"),
                    None,
                ) {
                    Ok(fp) => (fp, 0),
                    Err(e) => {
                        self.toast(&e.text());
                        return;
                    }
                }
            }
        };
        let Some(master) = self
            .session
            .keys
            .iter()
            .find(|k| k.master.fingerprint() == master_fp)
            .map(|k| &k.master)
        else {
            return;
        };
        let text = match master.account_xpub(script, account) {
            Ok(a) => {
                let shown = format!("{}", a.path())
                    .trim_start_matches("m/")
                    .replace('\'', "h");
                format!("[{}/{shown}]{}", a.master_fingerprint(), a.xpub())
            }
            Err(e) => {
                self.toast(&e.to_string());
                return;
            }
        };
        let kind = match script {
            ScriptType::Legacy => NewKind::Legacy,
            ScriptType::NestedSegwit => NewKind::NestedSegwit,
            ScriptType::NativeSegwit => NewKind::NativeSegwit,
            ScriptType::Taproot => NewKind::Taproot,
        };
        let name = format!(
            "{label} · {}",
            find.address.as_str().get(..10).unwrap_or("")
        );
        match self
            .session
            .add_wallet(&name, &kind.descriptor(1, &[text]), "Vanity search")
        {
            Ok(i) => {
                self.refresh_spend();
                self.vanity = None;
                self.wallet = i;
                self.screen = Screen::Wallets;
                self.toast(&format!("{name} is open"));
            }
            Err(e) => self.toast(&e.text()),
        }
    }

    /// Typing on the vanity screen: the prefix's characters, Backspace,
    /// Enter to go on. Returns whether the key was taken.
    pub(crate) fn vanity_key_in(&mut self, k: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
        if self.screen != Screen::Vanity {
            return false;
        }
        let net = self.session.network();
        let Some(v) = self.vanity.as_mut() else {
            return false;
        };
        match k {
            K::Escape => {
                self.screen = v.back;
                self.vanity = None;
            }
            K::Char(c) if v.open == Some(vstep::PREFIX) && !v.running => {
                if !v.grind.prefix.push(c, v.grind.script, net) {
                    v.error = Some(format!("{c} cannot stand there in this kind of address"));
                } else {
                    v.error = None;
                }
            }
            K::Backspace if v.open == Some(vstep::PREFIX) && !v.running => {
                v.grind.prefix.pop(v.grind.script, net);
                v.error = None;
            }
            K::Enter if v.open == Some(vstep::PREFIX) => {
                self.vanity_act(VanityAction::Next(vstep::PREFIX));
            }
            _ => return false,
        }
        true
    }
}

/// The fingerprint as the cards show it.
pub fn fp(fp: [u8; 4]) -> String {
    crate::wallet::fp_text(Fingerprint(fp))
}
